use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rscleaner_guard::{Guard, Intent, RuleCtx, VerifiedPath};
use rscleaner_model::{
    ByteSize, CancelToken, Item, Mode, PathRepr, Risk, Rule, RuleId, RuleStatus, SkipReason,
    Target, Totals,
};
use rscleaner_platform::{FileIdentity, Fs, FsMeta};

use crate::filter::Filters;

/// One planned item with the identity taken while scanning.
#[derive(Clone, Debug)]
pub struct ScanItem {
    pub item: Item,
    pub identity: FileIdentity,
}

/// A path that was left out of the plan and why.
#[derive(Clone, Debug)]
pub struct SkipEntry {
    pub path: PathRepr,
    pub reason: SkipReason,
}

/// The plan of one rule.
#[derive(Clone, Debug)]
pub struct RulePlan {
    pub rule_id: RuleId,
    pub risk: Risk,
    pub status: RuleStatus,
    pub items: Vec<ScanItem>,
    pub skips: Vec<SkipEntry>,
    pub totals: Totals,
}

impl RulePlan {
    fn new(rule: &Rule) -> Self {
        Self {
            rule_id: rule.id.clone(),
            risk: rule.risk,
            status: RuleStatus::NotApplicable,
            items: Vec::new(),
            skips: Vec::new(),
            totals: Totals::default(),
        }
    }

    fn push_skip(&mut self, path: &Path, reason: SkipReason) {
        self.skips.push(SkipEntry {
            path: PathRepr::from_path(path),
            reason,
        });
    }

    fn push(&mut self, item: Item, identity: FileIdentity) {
        self.totals.files = self.totals.files.saturating_add(item.files);
        self.totals.bytes_found = self.totals.bytes_found.saturating_add(item.size_apparent);
        if item.is_dir {
            self.totals.dirs = self.totals.dirs.saturating_add(1);
        }
        self.items.push(ScanItem { item, identity });
    }

    fn finish(mut self) -> Self {
        self.status = if !self.items.is_empty() {
            RuleStatus::Done
        } else if !self.skips.is_empty() {
            RuleStatus::Skipped
        } else {
            RuleStatus::NotApplicable
        };
        self
    }
}

/// A whole scan: one plan per rule plus the cancelled flag.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub rules: Vec<RulePlan>,
    pub totals: Totals,
    pub cancelled: bool,
}

impl Plan {
    pub fn from_rules(rules: Vec<RulePlan>, cancelled: bool) -> Self {
        let mut totals = Totals::default();
        for rule in &rules {
            totals.files = totals.files.saturating_add(rule.totals.files);
            totals.dirs = totals.dirs.saturating_add(rule.totals.dirs);
            totals.bytes_found = totals.bytes_found.saturating_add(rule.totals.bytes_found);
        }
        Self {
            rules,
            totals,
            cancelled,
        }
    }

    /// True when the plan holds at least one removable item.
    pub fn is_empty(&self) -> bool {
        self.rules.iter().all(|rule| rule.items.is_empty())
    }
}

/// Everything the planner needs: filesystem access, the guard, and a clock.
pub struct PlanContext<'a> {
    pub fs: &'a dyn Fs,
    pub guard: &'a Guard,
    pub cancel: CancelToken,
    pub now_unix: i64,
}

impl<'a> PlanContext<'a> {
    pub fn new(fs: &'a dyn Fs, guard: &'a Guard, cancel: CancelToken, now_unix: i64) -> Self {
        Self {
            fs,
            guard,
            cancel,
            now_unix,
        }
    }

    pub fn plan_rule(&self, rule: &Rule) -> RulePlan {
        let mut plan = RulePlan::new(rule);
        let mut seen = Seen::default();
        let mut applicable = false;

        for target in &rule.targets {
            if self.cancel.is_cancelled() {
                break;
            }
            let Target::Path {
                path,
                mode,
                include,
                exclude,
                min_age_days,
                min_size,
            } = target
            else {
                continue;
            };
            applicable = true;
            self.plan_path(
                &mut plan,
                &mut seen,
                path,
                *mode,
                include,
                exclude,
                *min_age_days,
                *min_size,
            );
        }

        if !applicable {
            plan.status = RuleStatus::NotApplicable;
            return plan;
        }
        plan.finish()
    }

    #[allow(clippy::too_many_arguments)]
    fn plan_path(
        &self,
        plan: &mut RulePlan,
        seen: &mut Seen,
        path: &str,
        mode: Mode,
        include: &[String],
        exclude: &[String],
        min_age_days: Option<u32>,
        min_size: Option<ByteSize>,
    ) {
        let target = Path::new(path);
        let ctx = RuleCtx {
            mode,
            include: include.to_vec(),
            min_age_days,
        };
        let filters = match Filters::build(
            include,
            exclude,
            min_age_days,
            min_size.map(|size| size.0),
            self.now_unix,
        ) {
            Ok(filters) => filters,
            Err(_) => {
                plan.push_skip(target, SkipReason::UnsupportedHere);
                return;
            }
        };
        let verified = match self.guard.verify(target, Intent::Delete, &ctx) {
            Ok(verified) => verified,
            Err(error) => {
                plan.push_skip(target, error.skip_reason());
                return;
            }
        };

        match mode {
            Mode::SelfItem => self.plan_self(plan, seen, &verified),
            Mode::Contents => self.plan_contents(plan, seen, &verified, &filters),
            Mode::Files => self.plan_files(plan, seen, &verified, &filters),
        }
    }

    fn plan_self(&self, plan: &mut RulePlan, seen: &mut Seen, verified: &VerifiedPath) {
        let Ok(meta) = self.fs.metadata(verified.final_path()) else {
            plan.push_skip(verified.given(), SkipReason::NotFound);
            return;
        };
        let (files, bytes) = self.measure(&meta, verified.final_path(), seen);
        let item = item_of(verified.final_path(), &meta, files, bytes);
        plan.push(item, meta.identity);
    }

    fn plan_contents(
        &self,
        plan: &mut RulePlan,
        seen: &mut Seen,
        verified: &VerifiedPath,
        filters: &Filters,
    ) {
        let entries = match self.fs.read_dir(verified.final_path()) {
            Ok(entries) => entries,
            Err(error) => {
                plan.push_skip(verified.given(), io_reason(&error));
                return;
            }
        };
        for entry in entries {
            if self.cancel.is_cancelled() {
                return;
            }
            let name = entry
                .path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            let modified = modified_unix(&entry.meta);
            if !filters.accepts_name(&name) || !filters.accepts_age(modified) {
                continue;
            }
            let (files, bytes) = self.measure(&entry.meta, &entry.path, seen);
            if !filters.accepts_size(bytes) {
                continue;
            }
            let ctx = RuleCtx {
                mode: Mode::Contents,
                include: Vec::new(),
                min_age_days: None,
            };
            match self.guard.verify(&entry.path, Intent::Delete, &ctx) {
                Ok(_) => {
                    let item = item_of(&entry.path, &entry.meta, files, bytes);
                    plan.push(item, entry.meta.identity);
                }
                Err(error) => plan.push_skip(&entry.path, error.skip_reason()),
            }
        }
    }

    fn plan_files(
        &self,
        plan: &mut RulePlan,
        seen: &mut Seen,
        verified: &VerifiedPath,
        filters: &Filters,
    ) {
        let ctx = RuleCtx {
            mode: Mode::Files,
            include: filters.include_names(),
            min_age_days: filters.min_age_days(),
        };
        let mut stack: Vec<PathBuf> = vec![verified.final_path().to_path_buf()];
        while let Some(dir) = stack.pop() {
            if self.cancel.is_cancelled() {
                return;
            }
            let entries = match self.fs.read_dir(&dir) {
                Ok(entries) => entries,
                Err(error) => {
                    plan.push_skip(&dir, io_reason(&error));
                    continue;
                }
            };
            for entry in entries {
                if self.cancel.is_cancelled() {
                    return;
                }
                if entry.meta.is_dir {
                    stack.push(entry.path);
                    continue;
                }
                let name = entry
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                let modified = modified_unix(&entry.meta);
                if !filters.accepts(&name, entry.meta.len, modified) {
                    continue;
                }
                match self.guard.verify(&entry.path, Intent::Delete, &ctx) {
                    Ok(_) => {
                        let (files, bytes) = self.measure(&entry.meta, &entry.path, seen);
                        let item = item_of(&entry.path, &entry.meta, files, bytes);
                        plan.push(item, entry.meta.identity);
                    }
                    Err(error) => plan.push_skip(&entry.path, error.skip_reason()),
                }
            }
        }
    }

    /// Files and bytes below a node. Links are never followed; a hard link
    /// contributes its bytes once.
    fn measure(&self, meta: &FsMeta, path: &Path, seen: &mut Seen) -> (u64, u64) {
        if meta.is_symlink || !meta.is_dir {
            let files = 1;
            let bytes = if seen.first(meta.identity) {
                meta.len
            } else {
                0
            };
            return (files, bytes);
        }

        let mut stack = vec![path.to_path_buf()];
        let mut files = 0u64;
        let mut bytes = 0u64;
        while let Some(dir) = stack.pop() {
            if self.cancel.is_cancelled() {
                break;
            }
            let Ok(entries) = self.fs.read_dir(&dir) else {
                continue;
            };
            for entry in entries {
                if entry.meta.is_symlink {
                    files = files.saturating_add(1);
                } else if entry.meta.is_dir {
                    stack.push(entry.path);
                } else {
                    files = files.saturating_add(1);
                    if seen.first(entry.meta.identity) {
                        bytes = bytes.saturating_add(entry.meta.len);
                    }
                }
            }
        }
        (files, bytes)
    }
}

fn item_of(path: &Path, meta: &FsMeta, files: u64, bytes: u64) -> Item {
    Item {
        path: PathRepr::from_path(path),
        size_apparent: bytes,
        size_on_disk: bytes,
        files,
        is_dir: meta.is_dir && !meta.is_symlink,
        modified_unix: modified_unix(meta),
        reparse: meta.is_symlink,
    }
}

fn modified_unix(meta: &FsMeta) -> Option<i64> {
    let modified = meta.modified?;
    let since = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    i64::try_from(since).ok()
}

fn io_reason(error: &std::io::Error) -> SkipReason {
    match error.kind() {
        std::io::ErrorKind::NotFound => SkipReason::NotFound,
        std::io::ErrorKind::PermissionDenied => SkipReason::AccessDenied,
        _ => SkipReason::UnsupportedHere,
    }
}

/// Identities already counted in this rule.
#[derive(Default)]
struct Seen {
    set: HashSet<(u64, u64)>,
}

impl Seen {
    fn first(&mut self, identity: FileIdentity) -> bool {
        self.set.insert((identity.volume, identity.file))
    }
}

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{Sender, bounded};
use rscleaner_exec::{CleanOptions, ExecError, ExecOutput, Executor};
use rscleaner_guard::{FsRemover, Guard};
use rscleaner_model::{
    CancelToken, Config, Event, Item, PlanId, Platform, Rule, RuleId, RuleStatus, RuleSummary,
    Totals, When,
};
use rscleaner_platform::{Elevation, Env, Fs, KnownFolders, Procs, SystemInfo};
use rscleaner_report::Journal;
use rscleaner_rules::set::{RuleSet, select};
use rscleaner_scan::{Plan, PlanContext};

use crate::job::Job;

const CHANNEL_CAPACITY: usize = 256;
const KEPT_PLANS: usize = 4;
const FIRST_ITEMS: usize = 5;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("unknown plan: {0}")]
    UnknownPlan(PlanId),
    #[error("the guard cannot start: {0}")]
    Guard(String),
    #[error("cannot open the journal: {0}")]
    Journal(#[from] rscleaner_report::JournalError),
    #[error("{0}")]
    Exec(String),
    #[error("the job panicked")]
    Panic,
}

pub struct EngineOptions {
    pub rules: RuleSet,
    pub config: Config,
    pub cwd: PathBuf,
    pub fs: Arc<dyn Fs>,
    pub env: Arc<dyn Env>,
    pub known: Arc<dyn KnownFolders>,
    pub procs: Arc<dyn Procs>,
    pub elevation: Arc<dyn Elevation>,
    pub system: Arc<dyn SystemInfo>,
    pub journal_path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct ScanRequest {
    pub profile: String,
    pub rule_masks: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CleanRequest {
    pub plan: PlanId,
    pub options: CleanOptions,
}

#[derive(Clone, Debug)]
pub struct ItemsPage {
    pub items: Vec<Item>,
    pub total: usize,
}

pub struct Engine {
    inner: Arc<Inner>,
}

struct Inner {
    options: EngineOptions,
    guard: Guard,
    plans: Mutex<VecDeque<(PlanId, Plan)>>,
    next_plan: AtomicU64,
}

impl Engine {
    pub fn open(options: EngineOptions) -> Result<Self, EngineError> {
        let guard = Guard::new(
            options.fs.clone(),
            options.env.clone(),
            options.known.clone(),
            options.cwd.clone(),
            &options.config.exclude_paths,
        )
        .map_err(|error| EngineError::Guard(error.to_string()))?;
        Ok(Self {
            inner: Arc::new(Inner {
                options,
                guard,
                plans: Mutex::new(VecDeque::new()),
                next_plan: AtomicU64::new(1),
            }),
        })
    }

    pub fn catalog(&self) -> Vec<Rule> {
        let mut rules: Vec<Rule> = self.inner.options.rules.rules.values().cloned().collect();
        rules.sort_by(|left, right| left.id.cmp(&right.id));
        rules
    }

    pub fn warnings(&self) -> Vec<rscleaner_model::RuleWarning> {
        self.inner.options.rules.warnings.clone()
    }

    pub fn settings(&self) -> Config {
        self.inner.options.config.clone()
    }

    pub fn scan(&self, request: ScanRequest) -> Job<Event, PlanId> {
        let (sender, receiver) = bounded(CHANNEL_CAPACITY);
        let cancel = CancelToken::new();
        let inner = self.inner.clone();
        let worker_cancel = cancel.clone();
        let worker = thread::spawn(move || inner.scan(&request, &sender, &worker_cancel));
        Job::new(receiver, cancel, worker)
    }

    pub fn clean(&self, request: CleanRequest) -> Job<Event, ExecOutput> {
        let (sender, receiver) = bounded(CHANNEL_CAPACITY);
        let cancel = CancelToken::new();
        let inner = self.inner.clone();
        let worker_cancel = cancel.clone();
        let worker = thread::spawn(move || inner.clean(&request, &sender, &worker_cancel));
        Job::new(receiver, cancel, worker)
    }

    pub fn plan_items(
        &self,
        plan: PlanId,
        rule: &RuleId,
        offset: usize,
        limit: usize,
    ) -> ItemsPage {
        let plans = match self.inner.plans.lock() {
            Ok(plans) => plans,
            Err(poisoned) => poisoned.into_inner(),
        };
        let Some((_, stored)) = plans.iter().find(|(id, _)| *id == plan) else {
            return ItemsPage {
                items: Vec::new(),
                total: 0,
            };
        };
        let Some(rule_plan) = stored.rules.iter().find(|entry| &entry.rule_id == rule) else {
            return ItemsPage {
                items: Vec::new(),
                total: 0,
            };
        };
        let total = rule_plan.items.len();
        let items = rule_plan
            .items
            .iter()
            .skip(offset)
            .take(limit)
            .map(|scan_item| scan_item.item.clone())
            .collect();
        ItemsPage { items, total }
    }

    pub fn plan(&self, plan: PlanId) -> Option<Plan> {
        let plans = match self.inner.plans.lock() {
            Ok(plans) => plans,
            Err(poisoned) => poisoned.into_inner(),
        };
        plans
            .iter()
            .find(|(id, _)| *id == plan)
            .map(|(_, stored)| stored.clone())
    }

    pub fn selected(&self, request: &ScanRequest) -> Vec<Rule> {
        self.inner.selected_rules(request)
    }
}

impl Inner {
    fn scan(
        &self,
        request: &ScanRequest,
        sender: &Sender<Event>,
        cancel: &CancelToken,
    ) -> Result<PlanId, EngineError> {
        let rules = self.selected_rules(request);
        let now = now_unix();
        let mut plans = Vec::new();
        let mut cancelled = false;

        for rule in rules {
            if cancel.is_cancelled() {
                cancelled = true;
                break;
            }
            let _ = sender.send(Event::RuleStarted {
                rule_id: rule.id.clone(),
            });
            let context = PlanContext::new(
                self.options.fs.as_ref(),
                self.options.env.as_ref(),
                self.options.known.as_ref(),
                &self.guard,
                self.options.cwd.clone(),
                cancel.clone(),
                now,
                self.options.elevation.is_elevated(),
            );
            let plan = context.plan_rule(&rule);
            let summary = RuleSummary {
                rule_id: rule.id.clone(),
                status: plan.status,
                risk: plan.risk,
                totals: plan.totals,
                skipped: plan.skips.len() as u64,
                failed: 0,
            };
            let _ = sender.send(Event::RuleScanned {
                summary,
                first_items: plan
                    .items
                    .iter()
                    .take(FIRST_ITEMS)
                    .map(|item| item.item.clone())
                    .collect(),
            });
            plans.push(plan);
        }

        let plan = Plan::from_rules(plans, cancelled || cancel.is_cancelled());
        let id = PlanId::new(self.next_plan.fetch_add(1, Ordering::SeqCst));
        {
            let mut stored = match self.plans.lock() {
                Ok(stored) => stored,
                Err(poisoned) => poisoned.into_inner(),
            };
            stored.push_back((id, plan.clone()));
            while stored.len() > KEPT_PLANS {
                stored.pop_front();
            }
        }
        let _ = sender.send(Event::ScanDone {
            plan_id: id,
            totals: plan.totals,
        });
        Ok(id)
    }

    fn clean(
        &self,
        request: &CleanRequest,
        sender: &Sender<Event>,
        cancel: &CancelToken,
    ) -> Result<ExecOutput, EngineError> {
        let plan = self
            .plan(request.plan)
            .ok_or(EngineError::UnknownPlan(request.plan))?;
        let rules = self.selected_rules(&ScanRequest::default());
        let mut journal = Journal::open(&self.options.journal_path)?;
        let remover = FsRemover::new(self.options.fs.clone());
        let mut emit = |event: Event| {
            let _ = sender.send(event);
        };
        let mut executor = Executor::new(
            &self.guard,
            self.options.fs.as_ref(),
            self.options.env.as_ref(),
            &remover,
            self.options.procs.as_ref(),
            self.options.elevation.as_ref(),
            &mut journal,
            cancel.clone(),
            request.options,
            now_unix(),
        );
        let output = executor
            .run(&plan, &rules, &mut emit)
            .map_err(|error| match error {
                ExecError::Journal(error) => EngineError::Journal(error),
                other => EngineError::Exec(other.to_string()),
            })?;
        let _ = sender.send(Event::ExecDone {
            summary: output.summary,
        });
        Ok(output)
    }

    fn plan(&self, id: PlanId) -> Option<Plan> {
        let plans = match self.plans.lock() {
            Ok(plans) => plans,
            Err(poisoned) => poisoned.into_inner(),
        };
        plans
            .iter()
            .find(|(stored, _)| *stored == id)
            .map(|(_, plan)| plan.clone())
    }

    fn selected_rules(&self, request: &ScanRequest) -> Vec<Rule> {
        let profile = if request.profile.is_empty() {
            "default"
        } else {
            request.profile.as_str()
        };
        let selected = select(&self.options.rules, &self.options.config, profile);
        let mut rules: Vec<Rule> = selected
            .rules
            .into_iter()
            .filter(|rule| rule.platforms.contains(&Platform::current()))
            .filter(|rule| self.when_applies(rule))
            .collect();

        for rule in self.options.rules.rules.values() {
            if !rule.platforms.contains(&Platform::current()) {
                continue;
            }
            if rules.iter().any(|kept| kept.id == rule.id) {
                continue;
            }
            if request
                .rule_masks
                .iter()
                .any(|mask| mask_matches(mask, rule.id.as_str()))
            {
                rules.push(rule.clone());
            }
        }

        if !request.rule_masks.is_empty() {
            rules.retain(|rule| {
                request
                    .rule_masks
                    .iter()
                    .any(|mask| mask_matches(mask, rule.id.as_str()))
            });
        }

        rules.sort_by(|left, right| left.id.cmp(&right.id));
        rules
    }

    fn when_applies(&self, rule: &Rule) -> bool {
        let Some(when) = &rule.when else {
            return true;
        };
        when_holds(
            when,
            self.options.fs.as_ref(),
            self.options.env.as_ref(),
            self.options.system.as_ref(),
        )
    }
}

fn when_holds(when: &When, fs: &dyn Fs, env: &dyn Env, system: &dyn SystemInfo) -> bool {
    if !when.distro.is_empty() {
        let os = system.os();
        let matches = when
            .distro
            .iter()
            .any(|name| os.id.iter().any(|id| id.eq_ignore_ascii_case(name)));
        if !matches {
            return false;
        }
    }
    if !when.path_exists.is_empty()
        && !when
            .path_exists
            .iter()
            .all(|path| fs.metadata(Path::new(path)).is_ok())
    {
        return false;
    }
    if !when.command_exists.is_empty() {
        let path = env
            .get("PATH")
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        let directories: Vec<PathBuf> = path
            .split(if cfg!(windows) { ';' } else { ':' })
            .filter(|entry| !entry.is_empty())
            .map(PathBuf::from)
            .collect();
        let found = when.command_exists.iter().all(|command| {
            directories.iter().any(|directory| {
                let candidate = directory.join(command);
                fs.metadata(&candidate).is_ok()
                    || fs.metadata(&candidate.with_extension("exe")).is_ok()
            })
        });
        if !found {
            return false;
        }
    }
    true
}

fn mask_matches(mask: &str, value: &str) -> bool {
    rscleaner_rules::glob::CompiledGlob::compile(mask)
        .map(|glob| glob.is_match(value))
        .unwrap_or(false)
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Rule ids and their statuses, for a summary line.
pub fn statuses(plan: &Plan) -> BTreeMap<RuleId, RuleStatus> {
    plan.rules
        .iter()
        .map(|rule| (rule.rule_id.clone(), rule.status))
        .collect()
}

/// Totals of a plan, for the CLI.
pub fn totals_of(plan: &Plan) -> Totals {
    plan.totals
}

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use rscleaner_model::{ErrorCode, PathRepr, Platform, Risk, RuleId, RuleStatus};

/// The report document. `schema` is always 1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: u32,
    pub platform: Platform,
    pub engine: String,
    pub size_kind: String,
    /// Start time in seconds since the Unix epoch.
    pub started_unix: i64,
    /// Finish time in seconds since the Unix epoch.
    pub finished_unix: i64,
    pub dry_run: bool,
    pub bytes_found: u64,
    pub bytes_freed: u64,
    pub bytes_trashed: u64,
    pub pending_reboot: Vec<PathRepr>,
    pub rules: Vec<RuleReport>,
}

impl Report {
    pub fn new(platform: Platform, engine: impl Into<String>, started_unix: i64) -> Self {
        Self {
            schema: 1,
            platform,
            engine: engine.into(),
            size_kind: "apparent".into(),
            started_unix,
            finished_unix: started_unix,
            dry_run: false,
            bytes_found: 0,
            bytes_freed: 0,
            bytes_trashed: 0,
            pending_reboot: Vec::new(),
            rules: Vec::new(),
        }
    }

    /// Recompute the document totals from its rules.
    pub fn recompute(&mut self) {
        self.bytes_found = 0;
        self.bytes_freed = 0;
        self.bytes_trashed = 0;
        for rule in &self.rules {
            self.bytes_found = self.bytes_found.saturating_add(rule.bytes_found);
            self.bytes_freed = self.bytes_freed.saturating_add(rule.bytes_freed);
            self.bytes_trashed = self.bytes_trashed.saturating_add(rule.bytes_trashed);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleReport {
    pub id: RuleId,
    pub status: RuleStatus,
    pub risk: Risk,
    pub bytes_found: u64,
    pub bytes_freed: u64,
    pub bytes_trashed: u64,
    pub removed: u64,
    pub skipped: Vec<SkipEntry>,
    pub failed: Vec<FailEntry>,
}

impl RuleReport {
    pub fn new(id: RuleId, status: RuleStatus, risk: Risk) -> Self {
        Self {
            id,
            status,
            risk,
            bytes_found: 0,
            bytes_freed: 0,
            bytes_trashed: 0,
            removed: 0,
            skipped: Vec::new(),
            failed: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkipEntry {
    pub path: PathRepr,
    pub reason: rscleaner_model::SkipReason,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FailEntry {
    pub path: PathRepr,
    pub code: ErrorCode,
    pub detail: String,
}

pub fn report_schema() -> schemars::Schema {
    schemars::schema_for!(Report)
}

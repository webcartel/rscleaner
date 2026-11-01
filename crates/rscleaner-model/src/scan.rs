use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::rule::PathRepr;

/// One entry of a plan: a file or a directory the rule would remove.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub path: PathRepr,
    pub size_apparent: u64,
    pub size_on_disk: u64,
    /// Number of files below the item, including the item itself for a file.
    pub files: u64,
    pub is_dir: bool,
    /// Modification time in seconds since the Unix epoch.
    pub modified_unix: Option<i64>,
    pub reparse: bool,
}

/// Why an item or a whole rule was left out of the plan or the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    InUse,
    AccessDenied,
    Protected,
    AppRunning,
    NeedsAdmin,
    OsTooOld,
    NotFound,
    NotApplicable,
    ChangedSinceScan,
    CloudPlaceholder,
    UnsupportedHere,
}

impl SkipReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InUse => "in_use",
            Self::AccessDenied => "access_denied",
            Self::Protected => "protected",
            Self::AppRunning => "app_running",
            Self::NeedsAdmin => "needs_admin",
            Self::OsTooOld => "os_too_old",
            Self::NotFound => "not_found",
            Self::NotApplicable => "not_applicable",
            Self::ChangedSinceScan => "changed_since_scan",
            Self::CloudPlaceholder => "cloud_placeholder",
            Self::UnsupportedHere => "unsupported_here",
        }
    }
}

/// Accumulated plan or run totals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Totals {
    pub files: u64,
    pub dirs: u64,
    pub bytes_found: u64,
}

impl Totals {
    pub fn add(&mut self, other: Self) {
        self.files = self.files.saturating_add(other.files);
        self.dirs = self.dirs.saturating_add(other.dirs);
        self.bytes_found = self.bytes_found.saturating_add(other.bytes_found);
    }
}

/// Identifier of a stored plan.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct PlanId(u64);
impl PlanId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for PlanId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Outcome of one rule in a plan or a report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuleStatus {
    Done,
    Skipped,
    Failed,
    DryRun,
    NotApplicable,
}

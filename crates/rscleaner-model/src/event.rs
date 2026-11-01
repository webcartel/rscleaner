use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, WarningCode};
use crate::ids::RuleId;
use crate::risk::Risk;
use crate::rule::PathRepr;
use crate::scan::{Item, PlanId, RuleStatus, SkipReason, Totals};

/// The first items of a rule, for a list that pages the rest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleSummary {
    pub rule_id: RuleId,
    pub status: RuleStatus,
    pub risk: Risk,
    pub totals: Totals,
    pub skipped: u64,
    pub failed: u64,
}

/// The outcome of a cleanup, without the per-item detail.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecSummary {
    pub removed: u64,
    pub skipped: u64,
    pub failed: u64,
    pub bytes_found: u64,
    pub bytes_freed: u64,
    pub bytes_trashed: u64,
    pub pending_reboot: u64,
}

/// A job event. Frontends localize the codes, never the strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    RuleStarted {
        rule_id: RuleId,
    },
    RuleProgress {
        rule_id: RuleId,
        files: u64,
        bytes: u64,
    },
    RuleScanned {
        summary: RuleSummary,
        first_items: Vec<Item>,
    },
    ScanDone {
        plan_id: PlanId,
        totals: Totals,
    },
    ItemRemoved {
        rule_id: RuleId,
        path: PathRepr,
        bytes: u64,
        to_trash: bool,
    },
    ItemSkipped {
        rule_id: RuleId,
        path: PathRepr,
        reason: SkipReason,
    },
    ItemFailed {
        rule_id: RuleId,
        path: PathRepr,
        code: ErrorCode,
        detail: String,
    },
    ExecDone {
        summary: ExecSummary,
    },
    Warning {
        rule_id: Option<RuleId>,
        code: WarningCode,
        detail: String,
    },
    Fatal {
        code: ErrorCode,
        detail: String,
    },
}

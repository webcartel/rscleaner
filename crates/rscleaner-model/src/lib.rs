#![forbid(unsafe_code)]

//! Serializable RSCleaner types and codes. No I/O lives here.

pub mod config;
pub mod error;
pub mod event;
pub mod ids;
pub mod job;
pub mod localized;
pub mod platform;
pub mod profile;
pub mod risk;
pub mod rule;
pub mod scan;

pub use config::{Config, RuleToggle, SpaceConfig, TrashConfig};
pub use error::{ErrorCode, RuleError, RuleWarning, WarningCode};
pub use event::{Event, ExecSummary, RuleSummary};
pub use ids::{RuleId, RuleIdError};
pub use job::CancelToken;
pub use localized::Localized;
pub use platform::{Mode, Platform, Theme};
pub use profile::Profile;
pub use risk::Risk;
pub use rule::{ByteSize, PathRepr, RegistryTarget, Rule, RuleFile, RuleSource, Target, When};
pub use scan::{Item, PlanId, RuleStatus, SkipReason, Totals};

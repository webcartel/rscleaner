#![forbid(unsafe_code)]

//! Report schema 1 and the append-only jsonl journal.

pub mod journal;
pub mod report;

pub use journal::{Journal, JournalAction, JournalEntry, JournalError, JournalResult};
pub use report::{FailEntry, Report, RuleReport, SkipEntry, report_schema};

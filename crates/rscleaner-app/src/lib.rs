#![forbid(unsafe_code)]

//! The Engine facade: the only entry point a frontend needs.

pub mod engine;
pub mod job;

pub use engine::{
    CleanRequest, Engine, EngineError, EngineOptions, ItemsPage, ScanRequest, statuses, totals_of,
};
pub use job::Job;

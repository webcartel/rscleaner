#![forbid(unsafe_code)]

//! The executor: a plan in, a report out. Every removal goes through the
//! guard and the journal.

pub mod command;
pub mod run;

pub use run::{CleanOptions, ExecError, ExecOutput, Executor};

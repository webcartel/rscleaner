#![forbid(unsafe_code)]

//! Path normalization, protection, and identity verification.

pub mod error;
pub mod guard;
pub mod normalize;
pub mod protected;
pub mod verified;

pub use error::{GuardError, GuardReason};
pub use guard::{Guard, RuleCtx};
pub use verified::{FsRemover, Intent, Remover, VerifiedPath};

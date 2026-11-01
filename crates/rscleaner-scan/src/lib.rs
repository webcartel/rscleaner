#![forbid(unsafe_code)]

//! The cleanup planner: rules in, a plan out. Nothing here deletes.

pub mod filter;
pub mod plan;

pub use plan::{Plan, PlanContext, RulePlan, ScanItem, SkipEntry};

#![forbid(unsafe_code)]

//! Rule loading, validation, merging, profiles, and configuration.

pub mod error;
pub mod glob;
pub mod load;
pub mod schema;
pub mod validate;
pub mod vars;

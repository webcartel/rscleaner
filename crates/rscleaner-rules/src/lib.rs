#![forbid(unsafe_code)]

//! Rule loading, validation, merging, profiles, and configuration.

pub mod builtin;
pub mod config_file;
pub mod error;
pub mod glob;
pub mod load;
pub mod paths;
pub mod schema;
pub mod set;
pub mod validate;
pub mod vars;

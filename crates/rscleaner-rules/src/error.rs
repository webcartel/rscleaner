use std::path::Path;

use rscleaner_model::{ErrorCode, RuleError};

pub fn yaml_error(path: &Path, error: serde_norway::Error) -> RuleError {
    let location = error.location();
    let message = error.to_string();
    let code = if message.contains("unknown field") {
        ErrorCode::UnknownField
    } else if message.contains("missing field")
        || message.contains("invalid type")
        || message.contains("invalid value")
    {
        ErrorCode::Schema
    } else {
        ErrorCode::Yaml
    };
    RuleError::new(
        path.display().to_string(),
        location.as_ref().map(|value| value.line()).unwrap_or(1),
        location.as_ref().map(|value| value.column()).unwrap_or(1),
        code,
        message,
    )
}

pub fn io_error(path: &Path, error: std::io::Error) -> RuleError {
    RuleError::new(
        path.display().to_string(),
        1,
        1,
        ErrorCode::Yaml,
        error.to_string(),
    )
}

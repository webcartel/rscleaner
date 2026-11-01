use std::fs;
use std::path::Path;

use rscleaner_model::{ErrorCode, RuleError, RuleFile};

use crate::error::{io_error, yaml_error};

pub fn load_file(path: &Path) -> Result<RuleFile, Vec<RuleError>> {
    let text = fs::read_to_string(path).map_err(|error| vec![io_error(path, error)])?;
    let file: RuleFile =
        serde_norway::from_str(&text).map_err(|error| vec![yaml_error(path, error)])?;

    if file.schema != 1 {
        return Err(vec![RuleError::new(
            path.display().to_string(),
            schema_line(&text),
            1,
            ErrorCode::Schema,
            format!("unsupported rule schema: {} (expected 1)", file.schema),
        )]);
    }

    Ok(file)
}

fn schema_line(text: &str) -> usize {
    text.lines()
        .position(|line| line.trim_start().starts_with("schema:"))
        .map(|index| index + 1)
        .unwrap_or(1)
}

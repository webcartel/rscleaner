use std::fs;
use std::path::Path;

use rscleaner_model::{Config, RuleError};

use crate::error::{io_error, yaml_error};

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_norway::Error),
}

pub fn load(path: &Path) -> Result<Config, Vec<RuleError>> {
    let text = fs::read_to_string(path).map_err(|error| vec![io_error(path, error)])?;
    serde_norway::from_str(&text).map_err(|error| vec![yaml_error(path, error)])
}

pub fn save(path: &Path, config: &Config) -> Result<(), SaveError> {
    let text = serde_norway::to_string(config)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

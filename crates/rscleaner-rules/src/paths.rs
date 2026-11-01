use std::path::{Path, PathBuf};

use directories::ProjectDirs;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppPaths {
    pub config: PathBuf,
    pub rules: PathBuf,
    pub logs: PathBuf,
    pub cache: PathBuf,
    pub registry_backups: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("cannot determine the application data directories")]
    NoProjectDirs,
    #[error("cannot determine the executable path")]
    NoExecutable,
}

pub fn resolve(data_dir: Option<&Path>, portable: bool) -> Result<AppPaths, PathError> {
    if let Some(base) = data_dir {
        return Ok(portable_paths(base));
    }

    if portable {
        let executable = std::env::current_exe().map_err(|_| PathError::NoExecutable)?;
        let parent = executable.parent().ok_or(PathError::NoExecutable)?;
        return Ok(portable_paths(&parent.join("data")));
    }

    let dirs = ProjectDirs::from("", "", "RSCleaner").ok_or(PathError::NoProjectDirs)?;
    let config =
        env_path("RSCLEANER_CONFIG").unwrap_or_else(|| dirs.config_dir().join("config.yaml"));
    let rules = env_path("RSCLEANER_RULES_DIR").unwrap_or_else(|| dirs.config_dir().join("rules"));
    let logs = dirs
        .state_dir()
        .unwrap_or_else(|| dirs.data_local_dir())
        .join("logs");
    let cache = dirs.cache_dir().to_path_buf();
    let registry_backups = dirs.data_local_dir().join("registry-backups");

    Ok(AppPaths {
        config,
        rules,
        logs,
        cache,
        registry_backups,
    })
}

fn portable_paths(base: &Path) -> AppPaths {
    AppPaths {
        config: base.join("config.yaml"),
        rules: base.join("rules"),
        logs: base.join("logs"),
        cache: base.join("cache"),
        registry_backups: base.join("registry-backups"),
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

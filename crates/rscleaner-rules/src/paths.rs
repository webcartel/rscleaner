use std::path::{Path, PathBuf};

use directories::BaseDirs;

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
    NoBaseDirs,
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

    let base = BaseDirs::new().ok_or(PathError::NoBaseDirs)?;
    let app = if cfg!(windows) {
        "RSCleaner"
    } else {
        "rscleaner"
    };

    let config_root = base.config_dir().join(app);
    let data_root = base.data_local_dir().join(app);
    let cache_root = if cfg!(windows) {
        data_root.join("cache")
    } else {
        base.cache_dir().join(app)
    };
    let state_root = base
        .state_dir()
        .map(|path| path.join(app))
        .unwrap_or_else(|| data_root.clone());

    let config = env_path("RSCLEANER_CONFIG").unwrap_or_else(|| config_root.join("config.yaml"));
    let rules = env_path("RSCLEANER_RULES_DIR").unwrap_or_else(|| config_root.join("rules"));

    Ok(AppPaths {
        config,
        rules,
        logs: state_root.join("logs"),
        cache: cache_root,
        registry_backups: data_root.join("registry-backups"),
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

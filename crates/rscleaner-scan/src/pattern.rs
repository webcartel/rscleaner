use std::path::{Path, PathBuf};

use globset::GlobBuilder;
use rscleaner_platform::{Env, KnownFolders};
use rscleaner_rules::vars::expand_vars;

/// Expand a declared `path` target: variables first, then any glob components.
/// A pattern with no matches yields an empty list; a missing known folder also
/// yields an empty list, because the rule simply does not apply here.
pub fn expand(
    pattern: &str,
    env: &dyn Env,
    known: &dyn KnownFolders,
    cwd: &Path,
    fs: &dyn rscleaner_platform::Fs,
) -> Vec<PathBuf> {
    let Ok(expanded) = expand_vars(pattern, env, known) else {
        return Vec::new();
    };
    if !expanded.missing_known.is_empty() {
        return Vec::new();
    }

    let text = if cfg!(windows) {
        expanded.path.replace('/', "\\")
    } else {
        expanded.path.clone()
    };
    let path = Path::new(&text);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    let mut prefix = PathBuf::new();
    let mut components: Vec<String> = Vec::new();
    for component in absolute.components() {
        if components.is_empty() && !is_glob(&component.as_os_str().to_string_lossy()) {
            prefix.push(component);
        } else {
            components.push(component.as_os_str().to_string_lossy().to_string());
        }
    }

    if components.is_empty() {
        return vec![prefix];
    }
    if !components.iter().any(|component| is_glob(component)) {
        let mut path = prefix;
        for component in components {
            path.push(component);
        }
        return vec![path];
    }

    let mut current = vec![prefix];
    for component in components {
        if !is_glob(&component) {
            current = current
                .into_iter()
                .map(|path| path.join(&component))
                .collect();
            continue;
        }
        let Ok(glob) = GlobBuilder::new(&component)
            .case_insensitive(cfg!(windows))
            .build()
        else {
            return Vec::new();
        };
        let matcher = glob.compile_matcher();
        let mut next = Vec::new();
        for directory in &current {
            let Ok(entries) = fs.read_dir(directory) else {
                continue;
            };
            for entry in entries {
                let name = entry
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                if matcher.is_match(&name) {
                    next.push(entry.path);
                }
            }
        }
        current = next;
    }
    current.retain(|path| fs.metadata(path).is_ok());
    current
}

fn is_glob(component: &str) -> bool {
    component
        .chars()
        .any(|character| matches!(character, '*' | '?' | '[' | '{'))
}

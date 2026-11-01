use std::path::{Path, PathBuf};

use rscleaner_platform::{Env, Fs, KnownFolder, KnownFolders};

use crate::error::{GuardError, GuardReason};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedPath {
    pub given: PathBuf,
    pub final_path: PathBuf,
    pub chain: Vec<PathBuf>,
    pub exists: bool,
}

pub fn normalize(
    fs: &dyn Fs,
    env: &dyn Env,
    known: &dyn KnownFolders,
    cwd: &Path,
    input: &Path,
) -> Result<NormalizedPath, GuardError> {
    let text = expand_text(&input.to_string_lossy(), env, known)?;
    let given = if cfg!(windows) {
        clean_windows(&text, cwd)?
    } else {
        clean_linux(&text, cwd)?
    };
    let long = fs
        .long_path(Path::new(&given))
        .unwrap_or_else(|_| PathBuf::from(&given));
    let exists = fs.metadata(&long).is_ok();
    Ok(NormalizedPath {
        given: long.clone(),
        final_path: long,
        chain: Vec::new(),
        exists,
    })
}

pub fn path_key(path: &str) -> String {
    if !cfg!(windows) {
        return if path == "/" {
            "/".into()
        } else {
            path.trim_end_matches('/').to_string()
        };
    }
    let normalized = path.replace('/', "\\");
    let trimmed = normalized.trim_end_matches('\\');
    trimmed.to_ascii_lowercase()
}

pub fn same_or_below(candidate: &str, root: &str) -> bool {
    if candidate == root {
        return true;
    }
    if !cfg!(windows) && root == "/" {
        return candidate.starts_with('/');
    }
    let separator = if cfg!(windows) { '\\' } else { '/' };
    candidate
        .strip_prefix(root)
        .is_some_and(|rest| rest.starts_with(separator))
}

fn expand_text(text: &str, env: &dyn Env, known: &dyn KnownFolders) -> Result<String, GuardError> {
    let mut output = String::new();
    let mut rest = text;

    if let Some(stripped) = rest.strip_prefix('~') {
        output.push_str(&env_string("HOME", env)?);
        rest = stripped;
    }

    while !rest.is_empty() {
        let next_brace = rest.find("${");
        let next_percent = rest.find('%');
        let (start, kind) = match (next_brace, next_percent) {
            (Some(brace), Some(percent)) => {
                if brace <= percent {
                    (brace, "${")
                } else {
                    (percent, "%")
                }
            }
            (Some(brace), None) => (brace, "${"),
            (None, Some(percent)) => (percent, "%"),
            (None, None) => {
                output.push_str(rest);
                break;
            }
        };
        output.push_str(&rest[..start]);
        let after = &rest[start + kind.len()..];
        let end = if kind == "${" {
            after.find('}')
        } else {
            after.find('%')
        }
        .ok_or_else(|| GuardError::new(GuardReason::InvalidPath, "unterminated variable"))?;
        let name = &after[..end];
        output.push_str(&resolve_variable(name, env, known)?);
        // The closing delimiter is one byte: `}` for `${...}`, `%` for `%...%`.
        rest = &after[end + 1..];
    }

    Ok(output)
}

fn resolve_variable(
    name: &str,
    env: &dyn Env,
    known: &dyn KnownFolders,
) -> Result<String, GuardError> {
    if let Some(folder) = name.strip_prefix("known:") {
        let folder = match folder {
            "Downloads" => KnownFolder::Downloads,
            "Documents" => KnownFolder::Documents,
            "Desktop" => KnownFolder::Desktop,
            "Pictures" => KnownFolder::Pictures,
            "Videos" => KnownFolder::Videos,
            "Music" => KnownFolder::Music,
            _ => {
                return Err(GuardError::new(
                    GuardReason::InvalidPath,
                    format!("unknown known folder: {folder}"),
                ));
            }
        };
        return known
            .get(folder)
            .map(|path| path.to_string_lossy().to_string())
            .ok_or_else(|| {
                GuardError::new(
                    GuardReason::InvalidPath,
                    format!("known folder is not set: {name}"),
                )
            });
    }
    env_string(name, env)
}

fn env_string(name: &str, env: &dyn Env) -> Result<String, GuardError> {
    env.get(name)
        .map(|value| value.to_string_lossy().to_string())
        .ok_or_else(|| {
            GuardError::new(
                GuardReason::InvalidPath,
                format!("variable is not set: {name}"),
            )
        })
}

fn clean_windows(text: &str, cwd: &Path) -> Result<String, GuardError> {
    let mut value = text.trim().replace('/', "\\");
    if let Some(stripped) = value.strip_prefix("\\\\?\\") {
        value = stripped.to_string();
    }
    if let Some(stripped) = value.strip_prefix("\\\\.\\") {
        value = stripped.to_string();
    }
    if value.starts_with("\\\\") {
        return Err(GuardError::new(
            GuardReason::Unsupported,
            "UNC paths are not supported",
        ));
    }

    if value.starts_with('\\') {
        let cwd_text = cwd.to_string_lossy();
        let drive = cwd_text.get(..2).unwrap_or("C:");
        value = format!("{drive}{value}");
    }
    if !is_windows_absolute(&value) {
        value = format!(
            "{}\\{}",
            cwd.to_string_lossy().trim_end_matches('\\'),
            value
        );
    }

    let mut parts = value.split('\\').filter(|part| !part.is_empty());
    let drive = parts.next().unwrap_or("C:").to_string();
    let mut components: Vec<String> = Vec::new();
    for part in parts {
        if part == "." {
            continue;
        }
        if part == ".." {
            components.pop();
            continue;
        }
        let cleaned = part.trim_end_matches(['.', ' ']);
        if cleaned.is_empty() {
            continue;
        }
        components.push(cleaned.to_string());
    }
    let mut result = format!("{drive}\\");
    result.push_str(&components.join("\\"));
    Ok(format!("\\\\?\\{result}"))
}

fn is_windows_absolute(value: &str) -> bool {
    value.len() >= 3
        && value.as_bytes()[1] == b':'
        && (value.as_bytes()[2] == b'\\' || value.as_bytes()[2] == b'/')
}

fn clean_linux(text: &str, cwd: &Path) -> Result<String, GuardError> {
    let value = if text.starts_with('/') {
        text.to_string()
    } else {
        format!("{}/{}", cwd.to_string_lossy().trim_end_matches('/'), text)
    };
    let mut components: Vec<String> = Vec::new();
    for part in value.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            components.pop();
            continue;
        }
        components.push(part.to_string());
    }
    Ok(format!("/{}", components.join("/")))
}

pub fn windows_prefix(value: &str) -> String {
    if cfg!(windows) {
        format!("\\\\?\\{}", value.trim_end_matches('\\'))
    } else {
        value.to_string()
    }
}

use rscleaner_platform::{Env, KnownFolder, KnownFolders};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpandError {
    UnknownVar { name: String },
    MalformedVariable,
}

impl std::fmt::Display for ExpandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVar { name } => write!(formatter, "unknown variable: {name}"),
            Self::MalformedVariable => formatter.write_str("malformed variable reference"),
        }
    }
}

impl std::error::Error for ExpandError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpandResult {
    pub path: String,
    pub missing_known: Vec<String>,
}

pub fn expand_vars(
    input: &str,
    env: &dyn Env,
    known: &dyn KnownFolders,
) -> Result<ExpandResult, ExpandError> {
    let mut output = String::new();
    let mut missing_known = Vec::new();
    let mut rest = input;

    if let Some(stripped) = rest.strip_prefix('~') {
        if let Some(home) = env.get("HOME") {
            output.push_str(&home.to_string_lossy());
        } else {
            missing_known.push("HOME".into());
            output.push('~');
        }
        rest = stripped;
    }

    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            return Err(ExpandError::MalformedVariable);
        };
        let name = &after[..end];
        if name.is_empty() {
            return Err(ExpandError::MalformedVariable);
        }

        if let Some(folder) = name.strip_prefix("known:") {
            let folder = known_folder(folder).ok_or_else(|| ExpandError::UnknownVar {
                name: name.to_string(),
            })?;
            if let Some(path) = known.get(folder) {
                output.push_str(&path.to_string_lossy());
            } else {
                missing_known.push(name.to_string());
                output.push_str(&format!("${{{name}}}"));
            }
        } else if let Some(value) = env.get(name) {
            output.push_str(&value.to_string_lossy());
        } else if is_known_env(name) {
            missing_known.push(name.to_string());
            output.push_str(&format!("${{{name}}}"));
        } else {
            return Err(ExpandError::UnknownVar {
                name: name.to_string(),
            });
        }

        rest = &after[end + 1..];
    }

    output.push_str(rest);
    Ok(ExpandResult {
        path: output,
        missing_known,
    })
}

fn known_folder(name: &str) -> Option<KnownFolder> {
    match name {
        "Downloads" => Some(KnownFolder::Downloads),
        "Documents" => Some(KnownFolder::Documents),
        "Desktop" => Some(KnownFolder::Desktop),
        "Pictures" => Some(KnownFolder::Pictures),
        "Videos" => Some(KnownFolder::Videos),
        "Music" => Some(KnownFolder::Music),
        _ => None,
    }
}

fn is_known_env(name: &str) -> bool {
    matches!(
        name,
        "LOCALAPPDATA"
            | "APPDATA"
            | "PROGRAMDATA"
            | "USERPROFILE"
            | "SYSTEMROOT"
            | "SYSTEMDRIVE"
            | "WINDIR"
            | "TEMP"
            | "TMPDIR"
            | "PROGRAMFILES"
            | "PROGRAMFILES(X86)"
            | "HOME"
            | "XDG_CACHE_HOME"
            | "XDG_CONFIG_HOME"
            | "XDG_DATA_HOME"
            | "XDG_STATE_HOME"
    )
}

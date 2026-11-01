use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use rscleaner_model::CancelToken;
use rscleaner_platform::{Env, Fs};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("the command is not in a trusted directory: {0}")]
    Untrusted(String),
    #[error("cannot start {cmd}: {detail}")]
    Spawn { cmd: String, detail: String },
    #[error("{0} did not finish within its timeout")]
    Timeout(String),
    #[error("cancelled while running {0}")]
    Cancelled(String),
    #[error("{cmd} exited with {code}: {output}")]
    Exit {
        cmd: String,
        code: i32,
        output: String,
    },
}

pub struct CommandSpec {
    pub cmd: String,
    pub args: Vec<String>,
    pub timeout_s: Option<u32>,
}

/// Run one `command` target: no shell, a cleared environment, a timeout, and
/// cancellation. Only trusted directories are searched for a bare name.
pub fn run(
    spec: &CommandSpec,
    env: &dyn Env,
    fs: &dyn Fs,
    cancel: &CancelToken,
) -> Result<String, CommandError> {
    let program =
        resolve(&spec.cmd, env, fs).ok_or_else(|| CommandError::Untrusted(spec.cmd.clone()))?;
    let timeout = Duration::from_secs(u64::from(spec.timeout_s.unwrap_or(600)));

    let mut child = Command::new(&program)
        .args(&spec.args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CommandError::Spawn {
            cmd: spec.cmd.clone(),
            detail: error.to_string(),
        })?;

    let (sender, receiver) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        let sender = sender.clone();
        std::thread::spawn(move || {
            let mut text = String::new();
            let mut stdout = stdout;
            let _ = stdout.read_to_string(&mut text);
            let _ = sender.send(text);
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let sender = sender.clone();
        std::thread::spawn(move || {
            let mut text = String::new();
            let mut stderr = stderr;
            let _ = stderr.read_to_string(&mut text);
            let _ = sender.send(text);
        });
    }
    drop(sender);

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                return Err(CommandError::Spawn {
                    cmd: spec.cmd.clone(),
                    detail: error.to_string(),
                });
            }
        }
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CommandError::Cancelled(spec.cmd.clone()));
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CommandError::Timeout(spec.cmd.clone()));
        }
        std::thread::sleep(Duration::from_millis(25));
    };

    let mut output = String::new();
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(text) => output.push_str(&text),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }

    if status.success() {
        Ok(output)
    } else {
        Err(CommandError::Exit {
            cmd: spec.cmd.clone(),
            code: status.code().unwrap_or(-1),
            output: first_lines(&output),
        })
    }
}

fn first_lines(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Resolve a bare name in trusted directories, or accept an absolute path that
/// already points into one of them.
pub fn trusted_dirs(env: &dyn Env) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(windows)]
    {
        let system_root = env
            .get("SYSTEMROOT")
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| r"C:\Windows".into());
        dirs.push(PathBuf::from(&system_root).join("System32"));
        dirs.push(PathBuf::from(&system_root).join("SysWOW64"));
        for name in ["PROGRAMFILES", "PROGRAMFILES(X86)", "PROGRAMW6432"] {
            if let Some(value) = env.get(name) {
                let text = value.to_string_lossy().to_string();
                if !text.trim().is_empty() {
                    dirs.push(PathBuf::from(text));
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = env;
        for directory in ["/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin"] {
            dirs.push(PathBuf::from(directory));
        }
    }
    dirs
}

fn resolve(name: &str, env: &dyn Env, fs: &dyn Fs) -> Option<PathBuf> {
    if Path::new(name).is_absolute() {
        return if is_trusted(name, env, fs) {
            Some(PathBuf::from(name))
        } else {
            None
        };
    }
    let dirs = trusted_dirs(env);
    let candidates: Vec<String> = if cfg!(windows) {
        vec![name.to_string(), format!("{name}.exe")]
    } else {
        vec![name.to_string()]
    };
    for directory in dirs {
        for candidate in &candidates {
            let path = directory.join(candidate);
            if fs.metadata(&path).is_ok() {
                return Some(path);
            }
        }
    }
    None
}

fn is_trusted(name: &str, env: &dyn Env, fs: &dyn Fs) -> bool {
    let path = Path::new(name);
    if !path.is_absolute() {
        return false;
    }
    let Ok(canonical) = fs.long_path(path) else {
        return false;
    };
    let canonical = strip_verbatim(&canonical.to_string_lossy());
    let canonical = Path::new(&canonical);
    trusted_dirs(env).iter().any(|dir| {
        let dir = strip_verbatim(&dir.to_string_lossy());
        let dir = Path::new(&dir);
        !dir.as_os_str().is_empty() && dir != canonical && canonical.starts_with(dir)
    })
}

fn strip_verbatim(text: &str) -> String {
    text.strip_prefix("\\\\?\\")
        .unwrap_or(text)
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

/// True when the rule declares at least one `command` target.
pub fn has_command_targets(rule: &rscleaner_model::Rule) -> bool {
    rule.targets
        .iter()
        .any(|target| matches!(target, rscleaner_model::Target::Command { .. }))
}

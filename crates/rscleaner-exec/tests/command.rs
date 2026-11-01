use rscleaner_exec::command::{CommandError, CommandSpec, run};
use rscleaner_model::CancelToken;
use rscleaner_platform::{Fs, SystemEnv};
use std::path::Path;

fn runner() -> (SystemEnv, Box<dyn Fs + Send + Sync>) {
    #[cfg(windows)]
    {
        (SystemEnv, Box::new(rscleaner_platform::WindowsFs::new()))
    }
    #[cfg(target_os = "linux")]
    {
        (SystemEnv, Box::new(rscleaner_platform::LinuxFs::new()))
    }
}

#[test]
fn a_trusted_command_runs_without_a_shell() {
    let (env, fs) = runner();
    let cmd = if cfg!(windows) {
        r"C:\Windows\System32\cmd.exe"
    } else {
        "/bin/sh"
    };
    let args = if cfg!(windows) {
        vec!["/C".to_string(), "echo hello".to_string()]
    } else {
        vec!["-c".to_string(), "echo hello".to_string()]
    };
    let spec = CommandSpec {
        cmd: cmd.into(),
        args,
        timeout_s: Some(10),
    };

    let output = run(&spec, &env, fs.as_ref(), &CancelToken::new())
        .unwrap_or_else(|error| panic!("command: {error}"));
    assert!(output.contains("hello"), "{output}");
}

#[test]
fn a_bare_trusted_name_is_found() {
    let (env, fs) = runner();
    let name = if cfg!(windows) { "cmd.exe" } else { "sh" };
    let args = if cfg!(windows) {
        vec!["/C".to_string(), "echo bare".to_string()]
    } else {
        vec!["-c".to_string(), "echo bare".to_string()]
    };
    let spec = CommandSpec {
        cmd: name.into(),
        args,
        timeout_s: Some(10),
    };

    let output = run(&spec, &env, fs.as_ref(), &CancelToken::new())
        .unwrap_or_else(|error| panic!("command: {error}"));
    assert!(output.contains("bare"), "{output}");
}

#[test]
fn a_command_outside_a_trusted_directory_is_refused() {
    let (env, fs) = runner();
    let dir = tempfile::tempdir().unwrap_or_else(|_| panic!("temp"));
    let name = if cfg!(windows) {
        dir.path().join("evil.exe")
    } else {
        dir.path().join("evil")
    };
    std::fs::write(&name, b"not a program").unwrap_or_else(|_| panic!("write"));
    let spec = CommandSpec {
        cmd: name.to_string_lossy().to_string(),
        args: Vec::new(),
        timeout_s: Some(5),
    };

    match run(&spec, &env, fs.as_ref(), &CancelToken::new()) {
        Err(CommandError::Untrusted(_)) => {}
        other => panic!("expected an untrusted error, got {other:?}"),
    }
    assert!(Path::new(&name).exists());
}

#[test]
fn a_non_zero_exit_is_reported_with_its_output() {
    let (env, fs) = runner();
    let cmd = if cfg!(windows) {
        r"C:\Windows\System32\cmd.exe"
    } else {
        "/bin/sh"
    };
    let args = if cfg!(windows) {
        vec!["/C".to_string(), "echo boom 1>&2 & exit 3".to_string()]
    } else {
        vec!["-c".to_string(), "echo boom 1>&2; exit 3".to_string()]
    };
    let spec = CommandSpec {
        cmd: cmd.into(),
        args,
        timeout_s: Some(10),
    };

    match run(&spec, &env, fs.as_ref(), &CancelToken::new()) {
        Err(CommandError::Exit { code, output, .. }) => {
            assert_eq!(code, 3);
            assert!(output.contains("boom"), "{output}");
        }
        other => panic!("expected an exit error, got {other:?}"),
    }
}

#[test]
fn a_command_past_its_timeout_is_killed() {
    let (env, fs) = runner();
    let cmd = if cfg!(windows) {
        r"C:\Windows\System32\cmd.exe"
    } else {
        "/bin/sh"
    };
    let args = if cfg!(windows) {
        vec![
            "/C".to_string(),
            r"C:\Windows\System32\ping.exe -n 20 127.0.0.1 > NUL".to_string(),
        ]
    } else {
        vec!["-c".to_string(), "sleep 20".to_string()]
    };
    let spec = CommandSpec {
        cmd: cmd.into(),
        args,
        timeout_s: Some(1),
    };

    match run(&spec, &env, fs.as_ref(), &CancelToken::new()) {
        Err(CommandError::Timeout(_)) => {}
        other => panic!("expected a timeout, got {other:?}"),
    }
}

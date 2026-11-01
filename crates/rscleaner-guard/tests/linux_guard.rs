#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rscleaner_guard::{Guard, Intent, RuleCtx};
use rscleaner_model::Mode;
use rscleaner_platform::{FakeEnv, FakeFs, FakeKnownFolders, KnownFolder, VolumeKind};

fn fixture() -> (Arc<FakeFs>, Guard) {
    let fs = Arc::new(FakeFs::new(VolumeKind::Fixed));
    for path in [
        "/etc/passwd",
        "/proc/1/status",
        "/tmp/old.tmp",
        "/var/log/syslog.1",
        "/home/me/.cache/data",
        "/home/me/Documents/report.txt",
    ] {
        fs.add_file(Path::new(path), b"x", 1);
    }
    let env = FakeEnv::from([("HOME", "/home/me")]);
    let known = FakeKnownFolders::from([(KnownFolder::Documents, "/home/me/Documents")]);
    let guard = match Guard::new(
        fs.clone(),
        Arc::new(env),
        Arc::new(known),
        PathBuf::from("/home/me"),
        &[],
    ) {
        Ok(guard) => guard,
        Err(error) => panic!("guard: {error}"),
    };
    (fs, guard)
}

#[test]
fn linux_system_and_user_data_are_protected() {
    let (_fs, guard) = fixture();
    for path in [
        "/etc/passwd",
        "/proc/1/status",
        "/home/me/Documents/report.txt",
    ] {
        assert!(
            guard
                .verify(Path::new(path), Intent::Delete, &RuleCtx::default())
                .is_err(),
            "expected protected: {path}"
        );
    }
}

#[test]
fn linux_tmp_logs_and_user_caches_need_filters_or_are_allowed() {
    let (_fs, guard) = fixture();
    let filtered = RuleCtx {
        mode: Mode::Files,
        include: vec!["*.tmp".into(), "*.1".into()],
        min_age_days: Some(7),
    };
    assert!(
        guard
            .verify(Path::new("/tmp/old.tmp"), Intent::Delete, &filtered)
            .is_ok()
    );
    assert!(
        guard
            .verify(Path::new("/var/log/syslog.1"), Intent::Delete, &filtered)
            .is_ok()
    );
    assert!(
        guard
            .verify(
                Path::new("/home/me/.cache/data"),
                Intent::Delete,
                &RuleCtx::default()
            )
            .is_ok()
    );
}

#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rscleaner_guard::{Guard, Intent, RuleCtx};
use rscleaner_platform::{FakeEnv, FakeFs, FakeKnownFolders, KnownFolder, VolumeKind};

#[test]
fn linux_table_has_at_least_one_hundred_protected_cases() {
    let fs = Arc::new(FakeFs::new(VolumeKind::Fixed));
    let mut paths = Vec::new();
    for index in 0..20 {
        paths.push(format!("/etc/config{index}.conf"));
        paths.push(format!("/usr/lib/library{index}.so"));
        paths.push(format!("/bin/tool{index}"));
        paths.push(format!("/proc/{index}/status"));
        paths.push(format!("/home/me/Documents/doc{index}.txt"));
    }
    assert_eq!(paths.len(), 100);
    for path in &paths {
        fs.add_file(Path::new(path), b"x", 1);
    }

    let env = FakeEnv::from([("HOME", "/home/me")]);
    let known = FakeKnownFolders::from([(KnownFolder::Documents, "/home/me/Documents")]);
    let guard = match Guard::new(
        fs,
        Arc::new(env),
        Arc::new(known),
        PathBuf::from("/home/me"),
        &[],
    ) {
        Ok(guard) => guard,
        Err(error) => panic!("guard: {error}"),
    };

    for path in paths {
        assert!(
            guard
                .verify(Path::new(&path), Intent::Delete, &RuleCtx::default())
                .is_err(),
            "expected protected: {path}"
        );
    }
}

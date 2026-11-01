#![cfg(target_os = "linux")]

use std::os::unix::fs::symlink;
use std::path::Path;
use std::sync::Arc;

use rscleaner_guard::{FsRemover, Guard, Intent, Remover, RuleCtx};
use rscleaner_model::Mode;
use rscleaner_platform::{LinuxFs, LinuxKnownFolders, SystemEnv};

#[test]
fn symlink_to_a_target_is_removed_without_touching_the_target() {
    let dir = tempfile::tempdir().expect("temp");
    let target = dir.path().join("target");
    std::fs::create_dir(&target).expect("target dir");
    let bait = target.join("bait.txt");
    std::fs::write(&bait, b"keep").expect("bait");
    let link = dir.path().join("link");
    symlink(&target, &link).expect("symlink");

    let fs = Arc::new(LinuxFs::new());
    let guard = match Guard::new(
        fs.clone(),
        Arc::new(SystemEnv),
        Arc::new(LinuxKnownFolders),
        dir.path().to_path_buf(),
        &[],
    ) {
        Ok(guard) => guard,
        Err(error) => panic!("guard: {error}"),
    };

    let contents = RuleCtx {
        mode: Mode::Contents,
        include: Vec::new(),
        min_age_days: None,
    };
    assert!(guard.verify(&link, Intent::Delete, &contents).is_err());

    let filtered = RuleCtx {
        mode: Mode::Files,
        include: vec!["*".into()],
        min_age_days: Some(1),
    };
    let verified = guard
        .verify(&link, Intent::Delete, &filtered)
        .expect("link itself is allowed");
    FsRemover::new(fs).remove(&verified).expect("remove link");
    assert!(!link.exists());
    assert!(bait.exists());
    assert!(Path::new(&bait).is_file());
}

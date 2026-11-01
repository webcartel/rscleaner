#![cfg(windows)]

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use rscleaner_guard::{FsRemover, Guard, Intent, Remover, RuleCtx};
use rscleaner_model::Mode;
use rscleaner_platform::{SystemEnv, WindowsFs, WindowsKnownFolders};

#[test]
fn junction_to_a_target_is_removed_without_touching_the_target() {
    let dir = tempfile::tempdir().expect("temp");
    let target = dir.path().join("target");
    std::fs::create_dir(&target).expect("target dir");
    let bait = target.join("bait.txt");
    std::fs::write(&bait, b"keep").expect("bait");
    let link = dir.path().join("junction");

    let status = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .status();
    let Ok(status) = status else {
        return;
    };
    if !status.success() {
        return;
    }

    let fs = Arc::new(WindowsFs::new());
    let guard = match Guard::new(
        fs.clone(),
        Arc::new(SystemEnv),
        Arc::new(WindowsKnownFolders),
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
    assert!(
        guard.verify(&link, Intent::Delete, &contents).is_err(),
        "contents through a junction must be refused"
    );

    let self_item = RuleCtx {
        mode: Mode::SelfItem,
        include: Vec::new(),
        min_age_days: None,
    };
    let verified = guard
        .verify(&link, Intent::Delete, &self_item)
        .expect("junction itself is allowed");
    FsRemover::new(fs)
        .remove(&verified)
        .expect("remove junction");
    assert!(!link.exists());
    assert!(bait.exists());
    assert!(Path::new(&bait).is_file());
}

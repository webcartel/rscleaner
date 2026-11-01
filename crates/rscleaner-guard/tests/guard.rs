#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rscleaner_guard::{FsRemover, Guard, Intent, Remover, RuleCtx};
use rscleaner_model::Mode;
use rscleaner_platform::{FakeEnv, FakeFs, FakeKnownFolders, Fs, VolumeKind};

fn fixture() -> (Arc<FakeFs>, Guard) {
    let fs = Arc::new(FakeFs::new(VolumeKind::Fixed));
    fs.add_dir(Path::new(r"C:\Windows"));
    fs.add_dir(Path::new(r"C:\Windows\System32"));
    fs.add_file(Path::new(r"C:\Windows\System32\probe.dll"), b"x", 1);
    fs.add_dir(Path::new(r"C:\Users"));
    fs.add_dir(Path::new(r"C:\Users\Me"));
    fs.add_dir(Path::new(r"C:\Users\Me\AppData"));
    fs.add_dir(Path::new(r"C:\Users\Me\AppData\Local"));
    fs.add_dir(Path::new(r"C:\Users\Me\AppData\Local\Temp"));
    fs.add_file(Path::new(r"C:\Users\Me\AppData\Local\Temp\a.tmp"), b"x", 1);
    fs.add_dir(Path::new(r"C:\Users\Me\Documents"));
    fs.add_file(Path::new(r"C:\Users\Me\Documents\report.docx"), b"x", 1);
    fs.add_file(Path::new(r"C:\Temp\link-target.txt"), b"x", 1);
    fs.add_symlink(
        Path::new(r"C:\Temp\link"),
        Path::new(r"C:\Users\Me\Documents\report.docx"),
    );

    let env = FakeEnv::from([
        ("SYSTEMROOT", r"C:\Windows"),
        ("USERPROFILE", r"C:\Users\Me"),
        ("APPDATA", r"C:\Users\Me\AppData\Roaming"),
        ("LOCALAPPDATA", r"C:\Users\Me\AppData\Local"),
        ("PROGRAMDATA", r"C:\ProgramData"),
        ("PROGRAMFILES", r"C:\Program Files"),
        ("PROGRAMFILES(X86)", r"C:\Program Files (x86)"),
    ]);
    let known = FakeKnownFolders::default();
    let guard = match Guard::new(
        fs.clone(),
        Arc::new(env),
        Arc::new(known),
        PathBuf::from(r"C:\Users\Me"),
        &[],
    ) {
        Ok(guard) => guard,
        Err(error) => panic!("guard: {error}"),
    };
    (fs, guard)
}

#[test]
fn protected_windows_paths_are_refused() {
    let (_fs, guard) = fixture();
    let error = guard
        .verify(
            Path::new(r"C:\Windows\System32\probe.dll"),
            Intent::Delete,
            &RuleCtx::default(),
        )
        .expect_err("must be protected");
    assert_eq!(error.reason, rscleaner_guard::GuardReason::Protected);
}

#[test]
fn user_data_requires_files_filters() {
    let (_fs, guard) = fixture();
    assert!(
        guard
            .verify(
                Path::new(r"C:\Users\Me\Documents\report.docx"),
                Intent::Delete,
                &RuleCtx::default(),
            )
            .is_err()
    );

    let ctx = RuleCtx {
        mode: Mode::Files,
        include: vec!["*.docx".into()],
        min_age_days: Some(30),
    };
    assert!(
        guard
            .verify(
                Path::new(r"C:\Users\Me\Documents\report.docx"),
                Intent::Delete,
                &ctx,
            )
            .is_ok()
    );
}

#[test]
fn reverify_detects_a_replaced_file() {
    let (fs, guard) = fixture();
    let verified = guard
        .verify(
            Path::new(r"C:\Users\Me\AppData\Local\Temp\a.tmp"),
            Intent::Delete,
            &RuleCtx::default(),
        )
        .expect("allowed");
    assert!(guard.reverify(&verified).is_ok());

    fs.add_file(Path::new(r"C:\Users\Me\AppData\Local\Temp\a.tmp"), b"y", 1);
    assert_eq!(
        guard.reverify(&verified).expect_err("changed").reason,
        rscleaner_guard::GuardReason::ChangedSinceScan
    );
}

#[test]
fn removing_a_symlink_keeps_its_target() {
    let (fs, guard) = fixture();
    let ctx = RuleCtx {
        mode: Mode::SelfItem,
        include: Vec::new(),
        min_age_days: None,
    };
    let verified = guard
        .verify(Path::new(r"C:\Temp\link"), Intent::Delete, &ctx)
        .expect("link itself is allowed");
    let remover = FsRemover::new(fs.clone());
    remover.remove(&verified).expect("remove link");
    assert!(fs.metadata(Path::new(r"C:\Temp\link")).is_err());
    assert!(
        fs.metadata(Path::new(r"C:\Users\Me\Documents\report.docx"))
            .is_ok()
    );
}

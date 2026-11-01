use std::path::PathBuf;

use proptest::prelude::*;
use rscleaner_guard::normalize::normalize;
use rscleaner_platform::{FakeEnv, FakeFs, FakeKnownFolders, VolumeKind};

fn cwd() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\Users\Me")
    } else {
        PathBuf::from("/tmp")
    }
}

proptest! {
    #[test]
    fn normalization_never_panics(input in ".*") {
        let fs = FakeFs::new(VolumeKind::Fixed);
        let env = FakeEnv::from([
            ("HOME", if cfg!(windows) { r"C:\Users\Me" } else { "/home/me" }),
            ("USERPROFILE", r"C:\Users\Me"),
            ("LOCALAPPDATA", r"C:\Users\Me\AppData\Local"),
        ]);
        let known = FakeKnownFolders::default();
        let _ = normalize(&fs, &env, &known, &cwd(), PathBuf::from(input).as_path());
    }
}

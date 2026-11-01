#![no_main]

use std::path::{Path, PathBuf};

use libfuzzer_sys::fuzz_target;
use rscleaner_guard::normalize::normalize;
use rscleaner_platform::{FakeEnv, FakeFs, FakeKnownFolders, VolumeKind};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let fs = FakeFs::new(VolumeKind::Fixed);
    let env = FakeEnv::from([
        ("HOME", "/home/me"),
        ("USERPROFILE", r"C:\Users\Me"),
        ("LOCALAPPDATA", r"C:\Users\Me\AppData\Local"),
    ]);
    let known = FakeKnownFolders::default();
    let cwd = if cfg!(windows) {
        PathBuf::from(r"C:\Users\Me")
    } else {
        PathBuf::from("/home/me")
    };
    let _ = normalize(&fs, &env, &known, &cwd, Path::new(&text));
});

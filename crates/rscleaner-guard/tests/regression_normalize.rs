use std::path::{Path, PathBuf};

use rscleaner_guard::normalize::normalize;
use rscleaner_platform::{FakeEnv, FakeFs, FakeKnownFolders, VolumeKind};

/// Input that libFuzzer found on Linux: two newlines, two 0xFF bytes, `b`,
/// a 0xFF byte, `${HOME}`, four 0xF6/0xFF bytes, and a NUL.
const CRASH: [u8; 17] = [
    10, 10, 255, 98, 255, 36, 123, 72, 79, 77, 69, 125, 246, 255, 255, 255, 0,
];

#[test]
fn the_fuzz_crash_input_does_not_panic() {
    let text = String::from_utf8_lossy(&CRASH).into_owned();
    let fs = FakeFs::new(VolumeKind::Fixed);
    let env = FakeEnv::from([("HOME", "/home/me")]);
    let known = FakeKnownFolders::default();
    let cwd = if cfg!(windows) {
        PathBuf::from(r"C:\Users\Me")
    } else {
        PathBuf::from("/home/me")
    };
    let _ = normalize(&fs, &env, &known, &cwd, Path::new(&text));
}

#[test]
fn a_variable_followed_by_a_wide_character_is_expanded() {
    let fs = FakeFs::new(VolumeKind::Fixed);
    let (home, separator) = if cfg!(windows) {
        (r"C:\Users\Me", "\\")
    } else {
        ("/home/me", "/")
    };
    let env = FakeEnv::from([("HOME", home)]);
    let known = FakeKnownFolders::default();
    let cwd = PathBuf::from(home);
    let plain = format!("{home}{separator}é");

    for input in [
        format!("${{HOME}}{separator}é"),
        format!("%HOME%{separator}é"),
    ] {
        let expanded =
            normalize(&fs, &env, &known, &cwd, Path::new(&input)).expect("variable expands");
        let expected =
            normalize(&fs, &env, &known, &cwd, Path::new(&plain)).expect("plain path normalizes");
        assert_eq!(expanded.given, expected.given, "{input}");
    }
}

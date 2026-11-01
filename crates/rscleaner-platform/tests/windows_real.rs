#![cfg(windows)]

use rscleaner_platform::{Fs, VolumeKind, WindowsFs};

#[test]
fn windows_metadata_reports_a_fixed_volume_and_identity() {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("probe.txt");
    std::fs::write(&path, b"probe").expect("write");
    let fs = WindowsFs::new();
    let meta = fs.metadata(&path).expect("metadata");
    assert!(meta.is_file);
    assert!(meta.identity.volume > 0);
    assert!(meta.identity.file > 0);
    assert_eq!(meta.volume, VolumeKind::Fixed);
}

#[test]
fn windows_long_path_keeps_an_existing_path() {
    let dir = tempfile::tempdir().expect("temp");
    let fs = WindowsFs::new();
    let long = fs.long_path(dir.path()).expect("long path");
    assert!(long.exists());
}

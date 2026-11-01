#![cfg(target_os = "linux")]

use rscleaner_platform::{Fs, LinuxFs, LinuxSystemInfo, SystemInfo};

#[test]
fn linux_metadata_reports_dev_and_ino() {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("probe.txt");
    std::fs::write(&path, b"probe").expect("write");
    let fs = LinuxFs::new();
    let meta = fs.metadata(&path).expect("metadata");
    assert!(meta.is_file);
    assert!(meta.identity.volume > 0);
    assert!(meta.identity.file > 0);
}

#[test]
fn linux_os_release_is_parsed() {
    let info = LinuxSystemInfo.os();
    assert!(!info.name.is_empty());
    assert!(!info.id.is_empty());
}

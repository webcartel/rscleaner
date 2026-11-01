use std::path::Path;

use rscleaner_platform::{FakeFs, Fs, VolumeKind};

#[test]
fn fake_fs_reports_identity_and_volume() {
    let fs = FakeFs::new(VolumeKind::Fixed);
    fs.add_file(Path::new(r"C:\Temp\a.tmp"), b"hello", 7);
    let meta = fs.metadata(Path::new(r"C:\Temp\a.tmp")).expect("metadata");
    assert_eq!(meta.len, 5);
    assert!(meta.is_file);
    assert_eq!(meta.identity.volume, 7);
    assert!(!meta.is_symlink);
}

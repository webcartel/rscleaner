use rscleaner_platform::{Env, FakeEnv, FakeKnownFolders, KnownFolder, KnownFolders};

#[test]
fn fake_env_and_known_folders_resolve_values() {
    let env = FakeEnv::from([("LOCALAPPDATA", r"C:\Users\me\AppData\Local")]);
    assert_eq!(
        env.get("LOCALAPPDATA").as_deref(),
        Some(std::ffi::OsStr::new(r"C:\Users\me\AppData\Local"))
    );
    let known = FakeKnownFolders::from([(KnownFolder::Downloads, r"C:\Users\me\Downloads")]);
    assert_eq!(
        known.get(KnownFolder::Downloads),
        Some(r"C:\Users\me\Downloads".into())
    );
}

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rscleaner_guard::{Guard, Intent, RuleCtx};
use rscleaner_model::Mode;
use rscleaner_platform::{FakeEnv, FakeFs, FakeKnownFolders, VolumeKind};

fn fixture() -> (Arc<FakeFs>, Guard) {
    let fs = Arc::new(FakeFs::new(VolumeKind::Fixed));
    let env = FakeEnv::from([
        ("SYSTEMROOT", r"C:\Windows"),
        ("USERPROFILE", r"C:\Users\Me"),
        ("APPDATA", r"C:\Users\Me\AppData\Roaming"),
        ("LOCALAPPDATA", r"C:\Users\Me\AppData\Local"),
        ("PROGRAMDATA", r"C:\ProgramData"),
        ("PROGRAMFILES", r"C:\Program Files"),
        ("PROGRAMFILES(X86)", r"C:\Program Files (x86)"),
    ]);
    let guard = match Guard::new(
        fs.clone(),
        Arc::new(env),
        Arc::new(FakeKnownFolders::default()),
        PathBuf::from(r"C:\Users\Me"),
        &[],
    ) {
        Ok(guard) => guard,
        Err(error) => panic!("guard: {error}"),
    };
    (fs, guard)
}

#[test]
fn protected_table_has_at_least_one_hundred_cases() {
    let (fs, guard) = fixture();
    let mut protected = Vec::new();
    for index in 0..20 {
        protected.push(format!(r"C:\Windows\System32\file{index}.dll"));
        protected.push(format!(r"C:\Program Files\App{index}\app.exe"));
        protected.push(format!(r"C:\Users\Other{index}\Documents\secret.txt"));
        protected.push(format!(r"C:\$Recycle.Bin\item{index}.tmp"));
        protected.push(format!(r"C:\Windows\WinSxS\probe{index}.dll"));
    }
    assert!(protected.len() >= 100);
    for path in &protected {
        fs.add_file(Path::new(path), b"x", 1);
        let result = guard.verify(Path::new(path), Intent::Delete, &RuleCtx::default());
        assert!(result.is_err(), "expected protected: {path}");
    }
}

#[test]
fn allowed_exceptions_and_filtered_user_data_pass() {
    let (fs, guard) = fixture();
    let mut allowed = Vec::new();
    for index in 0..10 {
        allowed.push(format!(r"C:\Users\Me\AppData\Local\Temp\cache{index}.tmp"));
        allowed.push(format!(r"C:\Windows\Temp\probe{index}.tmp"));
        allowed.push(format!(r"C:\Windows\Logs\CBS\probe{index}.log"));
        allowed.push(format!(
            r"C:\Windows\SoftwareDistribution\Download\update{index}.cab"
        ));
    }
    allowed.push(r"C:\Windows\MEMORY.DMP".into());
    for path in &allowed {
        fs.add_file(Path::new(path), b"x", 1);
        let result = guard.verify(Path::new(path), Intent::Delete, &RuleCtx::default());
        assert!(result.is_ok(), "expected allowed: {path}: {result:?}");
    }

    let filtered = RuleCtx {
        mode: Mode::Files,
        include: vec!["*.docx".into()],
        min_age_days: Some(30),
    };
    for index in 0..10 {
        let path = format!(r"C:\Users\Me\Documents\old{index}.docx");
        fs.add_file(Path::new(&path), b"x", 1);
        assert!(
            guard
                .verify(Path::new(&path), Intent::Delete, &filtered)
                .is_ok(),
            "expected filtered user data: {path}"
        );
    }
}

#[test]
fn browser_profile_only_allows_cache_names() {
    let (fs, guard) = fixture();
    fs.add_file(
        Path::new(r"C:\Users\Me\AppData\Local\Google\Chrome\User Data\Default\Cache\data_0"),
        b"x",
        1,
    );
    fs.add_file(
        Path::new(r"C:\Users\Me\AppData\Local\Google\Chrome\User Data\Default\Login Data"),
        b"x",
        1,
    );
    let ctx = RuleCtx {
        mode: Mode::SelfItem,
        include: Vec::new(),
        min_age_days: None,
    };
    assert!(
        guard
            .verify(
                Path::new(
                    r"C:\Users\Me\AppData\Local\Google\Chrome\User Data\Default\Cache\data_0"
                ),
                Intent::Delete,
                &ctx,
            )
            .is_ok()
    );
    let result = guard.verify(
        Path::new(r"C:\Users\Me\AppData\Local\Google\Chrome\User Data\Default\Login Data"),
        Intent::Delete,
        &ctx,
    );
    assert!(result.is_err());
}

#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use rscleaner_guard::Guard;
use rscleaner_model::{
    ByteSize, CancelToken, Localized, Mode, Platform, Risk, Rule, RuleId, RuleSource, RuleStatus,
    SkipReason, Target,
};
use rscleaner_platform::{FakeEnv, FakeFs, FakeKnownFolders, Fs, VolumeKind};
use rscleaner_scan::{PlanContext, RulePlan};

const NOW: i64 = 1_800_000_000;

fn fixture() -> (Arc<FakeFs>, Guard) {
    let fs = Arc::new(FakeFs::new(VolumeKind::Fixed));
    let env = FakeEnv::from([
        ("SYSTEMROOT", r"C:\Windows"),
        ("USERPROFILE", r"C:\Users\Me"),
        ("LOCALAPPDATA", r"C:\Users\Me\AppData\Local"),
        ("APPDATA", r"C:\Users\Me\AppData\Roaming"),
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

fn rule(targets: Vec<Target>) -> Rule {
    Rule {
        id: RuleId::from_str("test.rule").unwrap_or_else(|_| panic!("test.rule is a valid id")),
        name: Localized::text("Test"),
        description: None,
        category: "test".into(),
        risk: Risk::Safe,
        default_enabled: true,
        tags: Vec::new(),
        platforms: vec![Platform::Windows],
        when: None,
        requires_admin: false,
        requires_closed: Vec::new(),
        os_min_build: None,
        trash: None,
        targets,
        source: RuleSource::Builtin,
    }
}

fn path_target(path: &str, mode: Mode) -> Target {
    Target::Path {
        path: path.into(),
        mode,
        include: Vec::new(),
        exclude: Vec::new(),
        min_age_days: None,
        min_size: None,
    }
}

fn plan(guard: &Guard, fs: &Arc<FakeFs>, rule: &Rule) -> RulePlan {
    let env = FakeEnv::from([
        ("SYSTEMROOT", r"C:\Windows"),
        ("USERPROFILE", r"C:\Users\Me"),
        ("LOCALAPPDATA", r"C:\Users\Me\AppData\Local"),
    ]);
    let known = FakeKnownFolders::default();
    let context = PlanContext::new(
        fs.as_ref(),
        &env,
        &known,
        guard,
        PathBuf::from(r"C:\Users\Me"),
        CancelToken::new(),
        NOW,
        true,
    );
    context.plan_rule(rule)
}

#[test]
fn contents_mode_lists_children_and_sums_their_sizes() {
    let (fs, guard) = fixture();
    fs.add_dir(Path::new(r"C:\Temp\cache"));
    fs.add_file(Path::new(r"C:\Temp\cache\a.tmp"), &[0; 10], 1);
    fs.add_file(Path::new(r"C:\Temp\cache\b.tmp"), &[0; 5], 1);

    let plan = plan(
        &guard,
        &fs,
        &rule(vec![path_target(r"C:\Temp\cache", Mode::Contents)]),
    );

    assert_eq!(plan.items.len(), 2);
    assert_eq!(plan.totals.files, 2);
    assert_eq!(plan.totals.bytes_found, 15);
    assert_eq!(plan.status, RuleStatus::Done);
}

#[test]
fn self_mode_yields_the_target_with_its_subtree() {
    let (fs, guard) = fixture();
    fs.add_dir(Path::new(r"C:\Temp\cache"));
    fs.add_dir(Path::new(r"C:\Temp\cache\nested"));
    fs.add_file(Path::new(r"C:\Temp\cache\nested\a.tmp"), &[0; 7], 1);

    let plan = plan(
        &guard,
        &fs,
        &rule(vec![path_target(r"C:\Temp\cache", Mode::SelfItem)]),
    );

    assert_eq!(plan.items.len(), 1);
    let item = &plan.items[0].item;
    assert!(item.is_dir);
    assert_eq!(item.files, 1);
    assert_eq!(item.size_apparent, 7);
}

#[test]
fn files_mode_applies_include_exclude_age_and_size() {
    let (fs, guard) = fixture();
    fs.add_dir(Path::new(r"C:\Temp\logs"));
    fs.add_file(Path::new(r"C:\Temp\logs\old.log"), &[0; 2048], 1);
    fs.add_file(Path::new(r"C:\Temp\logs\keep.log"), &[0; 2048], 1);
    fs.add_file(Path::new(r"C:\Temp\logs\old.txt"), &[0; 2048], 1);
    fs.add_file(Path::new(r"C:\Temp\logs\tiny.log"), &[0; 10], 1);
    fs.add_file(Path::new(r"C:\Temp\logs\new.log"), &[0; 2048], 1);
    for name in ["old.log", "keep.log", "old.txt", "tiny.log"] {
        fs.set_modified(&Path::new(r"C:\Temp\logs").join(name), NOW - 90 * 86_400);
    }
    fs.set_modified(&Path::new(r"C:\Temp\logs").join("new.log"), NOW - 86_400);

    let rule = rule(vec![Target::Path {
        path: r"C:\Temp\logs".into(),
        mode: Mode::Files,
        include: vec!["*.log".into()],
        exclude: vec!["keep*".into()],
        min_age_days: Some(30),
        min_size: Some(ByteSize(1024)),
    }]);

    let plan = plan(&guard, &fs, &rule);
    let names: Vec<String> = plan
        .items
        .iter()
        .map(|item| item.item.path.display.clone())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
    assert!(names[0].ends_with("old.log"), "{names:?}");
}

#[test]
fn a_protected_child_is_a_skip_not_an_error() {
    let (fs, guard) = fixture();
    fs.add_dir(Path::new(r"C:\Windows\System32"));
    fs.add_file(Path::new(r"C:\Windows\System32\probe.dll"), &[0; 4], 1);

    let plan = plan(
        &guard,
        &fs,
        &rule(vec![path_target(
            r"C:\Windows\System32\probe.dll",
            Mode::SelfItem,
        )]),
    );

    assert!(plan.items.is_empty());
    assert_eq!(plan.skips.len(), 1);
    assert_eq!(plan.skips[0].reason, SkipReason::Protected);
    assert_eq!(plan.status, RuleStatus::Skipped);
}

#[test]
fn a_hard_link_contributes_its_bytes_once() {
    let (fs, guard) = fixture();
    fs.add_dir(Path::new(r"C:\Temp\links"));
    fs.add_file(Path::new(r"C:\Temp\links\a.bin"), &[0; 100], 1);
    fs.add_hard_link(
        Path::new(r"C:\Temp\links\b.bin"),
        Path::new(r"C:\Temp\links\a.bin"),
    );

    let plan = plan(
        &guard,
        &fs,
        &rule(vec![path_target(r"C:\Temp\links", Mode::Contents)]),
    );

    assert_eq!(plan.items.len(), 2);
    assert_eq!(plan.totals.files, 2);
    assert_eq!(plan.totals.bytes_found, 100);
}

#[test]
fn a_cancelled_scan_adds_no_items() {
    let (fs, guard) = fixture();
    fs.add_dir(Path::new(r"C:\Temp\cache"));
    fs.add_file(Path::new(r"C:\Temp\cache\a.tmp"), &[0; 10], 1);

    let cancel = CancelToken::new();
    cancel.cancel();
    let env = FakeEnv::from([("USERPROFILE", r"C:\Users\Me")]);
    let known = FakeKnownFolders::default();
    let context = PlanContext::new(
        fs.as_ref(),
        &env,
        &known,
        &guard,
        PathBuf::from(r"C:\Users\Me"),
        cancel,
        NOW,
        true,
    );
    let plan = context.plan_rule(&rule(vec![path_target(r"C:\Temp\cache", Mode::Contents)]));

    assert!(plan.items.is_empty());
}

#[test]
fn measurement_does_not_follow_a_link() {
    let (fs, guard) = fixture();
    fs.add_dir(Path::new(r"C:\Temp\cache"));
    fs.add_dir(Path::new(r"C:\Temp\elsewhere"));
    fs.add_file(Path::new(r"C:\Temp\elsewhere\keep.bin"), &[0; 50], 1);
    fs.add_symlink(
        Path::new(r"C:\Temp\cache\link"),
        Path::new(r"C:\Temp\elsewhere"),
    );

    let plan = plan(
        &guard,
        &fs,
        &rule(vec![path_target(r"C:\Temp\cache", Mode::SelfItem)]),
    );

    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].item.size_apparent, 0);
    assert!(
        fs.metadata(Path::new(r"C:\Temp\elsewhere\keep.bin"))
            .is_ok(),
        "the target of a link stays untouched"
    );
}

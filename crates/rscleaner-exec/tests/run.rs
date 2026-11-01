#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use rscleaner_exec::{CleanOptions, Executor};
use rscleaner_guard::{FsRemover, Guard};
use rscleaner_model::{
    CancelToken, Event, Localized, Mode, Platform, Risk, Rule, RuleId, RuleSource, RuleStatus,
    SkipReason, Target,
};
use rscleaner_platform::{
    FakeElevation, FakeEnv, FakeFs, FakeKnownFolders, FakeProcs, Fs, VolumeKind,
};
use rscleaner_report::Journal;
use rscleaner_scan::{Plan, PlanContext};

const NOW: i64 = 1_800_000_000;

struct Harness {
    fs: Arc<FakeFs>,
    guard: Guard,
    rules: Vec<Rule>,
}

fn harness(risk: Risk, requires_closed: Vec<String>, requires_admin: bool) -> Harness {
    let fs = Arc::new(FakeFs::new(VolumeKind::Fixed));
    fs.add_dir(Path::new(r"C:\Temp\cache"));
    fs.add_file(Path::new(r"C:\Temp\cache\a.tmp"), &[0; 10], 1);
    fs.add_file(Path::new(r"C:\Temp\cache\b.tmp"), &[0; 5], 1);

    let env = FakeEnv::from([
        ("SYSTEMROOT", r"C:\Windows"),
        ("USERPROFILE", r"C:\Users\Me"),
        ("LOCALAPPDATA", r"C:\Users\Me\AppData\Local"),
        ("APPDATA", r"C:\Users\Me\AppData\Roaming"),
        ("PROGRAMDATA", r"C:\ProgramData"),
        ("PROGRAMFILES", r"C:\Program Files"),
        ("PROGRAMFILES(X86)", r"C:\Program Files (x86)"),
    ]);
    let guard = Guard::new(
        fs.clone(),
        Arc::new(env),
        Arc::new(FakeKnownFolders::default()),
        PathBuf::from(r"C:\Users\Me"),
        &[],
    )
    .unwrap_or_else(|error| panic!("guard: {error}"));

    let rule = Rule {
        id: RuleId::from_str("test.rule").unwrap_or_else(|_| panic!("id")),
        name: Localized::text("Test"),
        description: None,
        category: "test".into(),
        risk,
        default_enabled: true,
        tags: Vec::new(),
        platforms: vec![Platform::Windows],
        when: None,
        requires_admin,
        requires_closed,
        os_min_build: None,
        trash: None,
        targets: vec![Target::Path {
            path: r"C:\Temp\cache".into(),
            mode: Mode::Contents,
            include: Vec::new(),
            exclude: Vec::new(),
            min_age_days: None,
            min_size: None,
        }],
        source: RuleSource::Builtin,
    };

    Harness {
        fs,
        guard,
        rules: vec![rule],
    }
}

fn scan(harness: &Harness) -> Plan {
    let env = FakeEnv::from([("USERPROFILE", r"C:\Users\Me")]);
    let known = FakeKnownFolders::default();
    let context = PlanContext::new(
        harness.fs.as_ref(),
        &env,
        &known,
        &harness.guard,
        PathBuf::from(r"C:\Users\Me"),
        CancelToken::new(),
        NOW,
        true,
    );
    let rules = harness
        .rules
        .iter()
        .map(|rule| context.plan_rule(rule))
        .collect();
    Plan::from_rules(rules, false)
}

struct Fixture {
    harness: Harness,
    journal_path: PathBuf,
    _dir: tempfile::TempDir,
}

fn fixture(risk: Risk, requires_closed: Vec<String>, requires_admin: bool) -> Fixture {
    let dir = tempfile::tempdir().unwrap_or_else(|_| panic!("temp"));
    Fixture {
        harness: harness(risk, requires_closed, requires_admin),
        journal_path: dir.path().join("journal.jsonl"),
        _dir: dir,
    }
}

fn run(
    fixture: &mut Fixture,
    plan: &Plan,
    options: CleanOptions,
    procs: &FakeProcs,
    elevation: &FakeElevation,
) -> (rscleaner_exec::ExecOutput, Vec<Event>) {
    let remover = FsRemover::new(fixture.harness.fs.clone());
    let mut journal = Journal::open(&fixture.journal_path).unwrap_or_else(|_| panic!("journal"));
    let mut events = Vec::new();
    let env = FakeEnv::from([("SYSTEMROOT", r"C:\Windows")]);
    let mut executor = Executor::new(
        &fixture.harness.guard,
        fixture.harness.fs.as_ref(),
        &env,
        &remover,
        procs,
        elevation,
        &mut journal,
        CancelToken::new(),
        options,
        NOW,
    );
    let output = executor
        .run(plan, &fixture.harness.rules, &mut |event| {
            events.push(event)
        })
        .unwrap_or_else(|error| panic!("run: {error}"));
    (output, events)
}

#[test]
fn a_dry_run_reports_but_removes_nothing() {
    let mut f = fixture(Risk::Safe, Vec::new(), false);
    let plan = scan(&f.harness);

    let (output, events) = run(
        &mut f,
        &plan,
        CleanOptions {
            dry_run: true,
            ..CleanOptions::default()
        },
        &FakeProcs::default(),
        &FakeElevation::default(),
    );

    assert!(
        f.harness
            .fs
            .metadata(Path::new(r"C:\Temp\cache\a.tmp"))
            .is_ok()
    );
    assert_eq!(output.report.rules[0].status, RuleStatus::DryRun);
    assert_eq!(output.report.bytes_found, 15);
    assert_eq!(output.report.bytes_freed, 0);
    assert_eq!(output.remaining, 0);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ItemRemoved { .. }))
    );
}

#[test]
fn a_real_run_removes_files_and_writes_the_journal() {
    let mut f = fixture(Risk::Safe, Vec::new(), false);
    let plan = scan(&f.harness);

    let (output, events) = run(
        &mut f,
        &plan,
        CleanOptions::default(),
        &FakeProcs::default(),
        &FakeElevation::default(),
    );

    assert!(
        f.harness
            .fs
            .metadata(Path::new(r"C:\Temp\cache\a.tmp"))
            .is_err()
    );
    assert_eq!(output.report.bytes_freed, 15);
    assert_eq!(output.report.rules[0].removed, 2);
    assert_eq!(output.report.rules[0].status, RuleStatus::Done);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::ItemRemoved { .. }))
            .count(),
        2
    );
    let journal = std::fs::read_to_string(&f.journal_path).unwrap_or_else(|_| panic!("journal"));
    assert_eq!(journal.lines().count(), 2);
}

#[test]
fn a_running_process_skips_the_whole_rule() {
    let mut f = fixture(Risk::Safe, vec!["game.exe".into()], false);
    let plan = scan(&f.harness);
    let procs = FakeProcs::new(["game.exe".to_string()]);

    let (output, _) = run(
        &mut f,
        &plan,
        CleanOptions::default(),
        &procs,
        &FakeElevation::default(),
    );

    assert_eq!(output.report.rules[0].status, RuleStatus::Skipped);
    assert_eq!(output.report.rules[0].skipped.len(), 2);
    assert_eq!(
        output.report.rules[0].skipped[0].reason,
        SkipReason::AppRunning
    );
    assert!(
        f.harness
            .fs
            .metadata(Path::new(r"C:\Temp\cache\a.tmp"))
            .is_ok()
    );
}

#[test]
fn an_admin_rule_without_elevation_is_skipped() {
    let mut f = fixture(Risk::Safe, Vec::new(), true);
    let plan = scan(&f.harness);

    let (output, _) = run(
        &mut f,
        &plan,
        CleanOptions::default(),
        &FakeProcs::default(),
        &FakeElevation::new(false),
    );

    assert_eq!(
        output.report.rules[0].skipped[0].reason,
        SkipReason::NeedsAdmin
    );
    assert_eq!(output.remaining, 2);
}

#[test]
fn an_elevated_admin_rule_runs() {
    let mut f = fixture(Risk::Safe, Vec::new(), true);
    let plan = scan(&f.harness);

    let (output, _) = run(
        &mut f,
        &plan,
        CleanOptions::default(),
        &FakeProcs::default(),
        &FakeElevation::new(true),
    );

    assert_eq!(output.report.rules[0].removed, 2);
}

#[test]
fn a_dangerous_rule_always_uses_the_trash() {
    let mut f = fixture(Risk::Dangerous, Vec::new(), false);
    let plan = scan(&f.harness);

    let (output, _) = run(
        &mut f,
        &plan,
        CleanOptions {
            use_trash: false,
            ..CleanOptions::default()
        },
        &FakeProcs::default(),
        &FakeElevation::default(),
    );

    assert_eq!(output.report.bytes_freed, 0);
    assert_eq!(output.report.bytes_trashed, 15);
}

#[test]
fn a_replaced_file_is_not_removed() {
    let mut f = fixture(Risk::Safe, Vec::new(), false);
    let plan = scan(&f.harness);
    f.harness
        .fs
        .add_file(Path::new(r"C:\Temp\cache\a.tmp"), &[0; 10], 1);

    let (output, _) = run(
        &mut f,
        &plan,
        CleanOptions::default(),
        &FakeProcs::default(),
        &FakeElevation::default(),
    );

    assert_eq!(output.report.rules[0].removed, 1);
    assert_eq!(output.report.rules[0].skipped.len(), 1);
    assert_eq!(
        output.report.rules[0].skipped[0].reason,
        SkipReason::ChangedSinceScan
    );
}

#[test]
fn a_command_only_rule_runs_and_is_journaled() {
    let mut f = fixture(Risk::Safe, Vec::new(), false);
    let command = r"C:\Windows\System32\cmd.exe";
    f.harness.rules[0].targets = vec![Target::Command {
        cmd: command.into(),
        args: vec!["/C".into(), "echo ok".into()],
        timeout_s: Some(10),
    }];
    let plan = scan(&f.harness);

    let (output, events) = run(
        &mut f,
        &plan,
        CleanOptions::default(),
        &FakeProcs::default(),
        &FakeElevation::default(),
    );

    assert_eq!(output.report.rules[0].status, RuleStatus::Done);
    assert_eq!(output.report.rules[0].failed.len(), 0);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Warning { detail, .. } if detail.contains("ok")
    )));
    let journal = std::fs::read_to_string(&f.journal_path).unwrap_or_else(|_| panic!("journal"));
    assert!(journal.contains("\"action\":\"command\""), "{journal}");
}

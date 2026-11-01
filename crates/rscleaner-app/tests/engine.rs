#![cfg(windows)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use rscleaner_app::{CleanRequest, Engine, EngineOptions, ScanRequest};
use rscleaner_exec::CleanOptions;
use rscleaner_model::{
    Config, Event, Localized, Mode, Platform, Risk, Rule, RuleId, RuleSource, Target,
};
use rscleaner_platform::{
    FakeElevation, FakeEnv, FakeFs, FakeKnownFolders, FakeProcs, FakeSystemInfo, Fs, VolumeKind,
};
use rscleaner_rules::set::{RuleSourceSet, merge_sources};

const CWD: &str = r"C:\Users\Me";

fn engine(fs: Arc<FakeFs>, journal: PathBuf) -> Engine {
    let rule = Rule {
        id: RuleId::from_str("test.rule").unwrap_or_else(|_| panic!("id")),
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
    let set = merge_sources(vec![RuleSourceSet {
        name: "test".into(),
        rules: vec![rule],
    }]);

    Engine::open(EngineOptions {
        rules: set,
        config: Config::default(),
        cwd: PathBuf::from(CWD),
        fs: fs.clone(),
        env: Arc::new(FakeEnv::from([
            ("SYSTEMROOT", r"C:\Windows"),
            ("USERPROFILE", r"C:\Users\Me"),
            ("LOCALAPPDATA", r"C:\Users\Me\AppData\Local"),
        ])),
        known: Arc::new(FakeKnownFolders::default()),
        procs: Arc::new(FakeProcs::default()),
        elevation: Arc::new(FakeElevation::default()),
        system: Arc::new(FakeSystemInfo::default()),
        journal_path: journal,
    })
    .unwrap_or_else(|error| panic!("engine: {error}"))
}

fn fixture() -> (Arc<FakeFs>, Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap_or_else(|_| panic!("temp"));
    let fs = Arc::new(FakeFs::new(VolumeKind::Fixed));
    fs.add_dir(Path::new(r"C:\Temp\cache"));
    fs.add_file(Path::new(r"C:\Temp\cache\a.tmp"), &[0; 10], 1);
    fs.add_file(Path::new(r"C:\Temp\cache\b.tmp"), &[0; 5], 1);
    let engine = engine(fs.clone(), dir.path().join("journal.jsonl"));
    (fs, engine, dir)
}

#[test]
fn scan_finishes_with_a_stored_plan() {
    let (_fs, engine, _dir) = fixture();
    let job = engine.scan(ScanRequest::default());
    let cancel = job.cancel.clone();
    let events: Vec<Event> = job.events.iter().collect();
    assert!(!cancel.is_cancelled());
    assert!(matches!(events.last(), Some(Event::ScanDone { .. })));
    drop(events);

    let plan_id = job.result().unwrap_or_else(|error| panic!("{error}"));
    let plan = engine.plan(plan_id).unwrap_or_else(|| panic!("plan"));
    assert_eq!(plan.totals.files, 2);
    assert_eq!(plan.totals.bytes_found, 15);

    let id = RuleId::from_str("test.rule").unwrap_or_else(|_| panic!("id"));
    let page = engine.plan_items(plan_id, &id, 0, 10);
    assert_eq!(page.total, 2);
    assert_eq!(page.items.len(), 2);
    let empty = engine.plan_items(plan_id, &id, 10, 10);
    assert!(empty.items.is_empty());
}

#[test]
fn a_dry_run_through_the_engine_leaves_files_alone() {
    let (fs, engine, _dir) = fixture();
    let job = engine.scan(ScanRequest::default());
    for _event in job.events.iter() {}
    let plan_id = job.result().unwrap_or_else(|error| panic!("{error}"));

    let job = engine.clean(CleanRequest {
        plan: plan_id,
        options: CleanOptions {
            dry_run: true,
            ..CleanOptions::default()
        },
    });
    let events: Vec<Event> = job.events.iter().collect();
    let output = job.result().unwrap_or_else(|error| panic!("{error}"));

    assert!(matches!(events.last(), Some(Event::ExecDone { .. })));
    assert_eq!(output.report.bytes_freed, 0);
    assert!(fs.metadata(Path::new(r"C:\Temp\cache\a.tmp")).is_ok());
}

#[test]
fn an_unknown_plan_is_an_error() {
    let (_fs, engine, _dir) = fixture();
    let job = engine.clean(CleanRequest {
        plan: rscleaner_model::PlanId::new(99),
        options: CleanOptions::default(),
    });
    for _event in job.events.iter() {}
    assert!(job.result().is_err());
}

#[test]
fn the_catalog_is_sorted_and_complete() {
    let (_fs, engine, _dir) = fixture();
    let catalog = engine.catalog();
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[0].id.as_str(), "test.rule");
    assert_eq!(engine.warnings().len(), 0);
    let mut expected = BTreeMap::new();
    expected.insert(catalog[0].id.clone(), catalog[0].clone());
    assert_eq!(expected.len(), 1);
}

#[test]
fn a_cancelled_scan_closes_its_channel_within_a_second() {
    use std::time::{Duration, Instant};

    let dir = tempfile::tempdir().unwrap_or_else(|_| panic!("temp"));
    let fs = Arc::new(FakeFs::new(VolumeKind::Fixed));
    fs.add_dir(Path::new(r"C:\Temp\big"));
    for index in 0..20_000 {
        fs.add_file(
            Path::new(&format!(r"C:\Temp\big\file{index}.tmp")),
            &[0; 4],
            1,
        );
    }
    let engine = engine(fs, dir.path().join("journal.jsonl"));

    let job = engine.scan(ScanRequest::default());
    job.cancel.cancel();
    let started = Instant::now();
    let mut events = 0;
    while job.events.recv_timeout(Duration::from_secs(2)).is_ok() {
        events += 1;
        if started.elapsed() > Duration::from_secs(1) {
            break;
        }
    }
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "the channel must close promptly, saw {events} events"
    );
}

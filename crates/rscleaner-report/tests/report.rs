use rscleaner_model::{ErrorCode, PathRepr, Platform, Risk, RuleId, RuleStatus, SkipReason};
use rscleaner_report::{
    FailEntry, Journal, JournalAction, JournalEntry, JournalResult, Report, RuleReport, SkipEntry,
};

fn rule_id(value: &str) -> RuleId {
    value
        .parse()
        .unwrap_or_else(|_| panic!("{value} is a valid id"))
}

#[test]
fn report_round_trips_and_keeps_schema_one() {
    let mut report = Report::new(Platform::Windows, "0.1.0", 1_791_000_000);
    report.dry_run = true;
    let mut rule = RuleReport::new(rule_id("system.temp"), RuleStatus::DryRun, Risk::Safe);
    rule.bytes_found = 10;
    rule.removed = 1;
    rule.skipped.push(SkipEntry {
        path: PathRepr::text(r"C:\Temp\busy.tmp"),
        reason: SkipReason::InUse,
    });
    rule.failed.push(FailEntry {
        path: PathRepr::text(r"C:\Temp\bad.tmp"),
        code: ErrorCode::Io,
        detail: "denied".into(),
    });
    report.rules.push(rule);
    report.recompute();

    let text = serde_json::to_string(&report).unwrap_or_else(|_| panic!("serialize"));
    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|_| panic!("parse"));

    assert_eq!(value["schema"], 1);
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["bytes_found"], 10);
    assert!(value["bytes_freed"].is_number());
    assert!(value["bytes_trashed"].is_number());
    assert!(value["pending_reboot"].is_array());
    assert_eq!(value["rules"][0]["status"], "dry_run");
    assert_eq!(value["rules"][0]["skipped"][0]["reason"], "in_use");
}

#[test]
fn the_journal_writes_one_json_object_per_line() {
    let dir = tempfile::tempdir().unwrap_or_else(|_| panic!("temp"));
    let path = dir.path().join("logs").join("2026-11-01.jsonl");
    let mut journal = Journal::open(&path).unwrap_or_else(|_| panic!("open"));

    for _ in 0..2 {
        let entry = JournalEntry {
            time_unix: 1_791_000_000,
            rule: rule_id("system.temp"),
            path: PathRepr::text(r"C:\Temp\a.tmp"),
            action: JournalAction::Remove,
            bytes: 4,
            result: JournalResult::Ok,
        };
        journal.append(&entry).unwrap_or_else(|_| panic!("append"));
    }

    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read"));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in lines {
        let value: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|_| panic!("json"));
        assert_eq!(value["action"], "remove");
        assert_eq!(value["path"]["display"], r"C:\Temp\a.tmp");
    }
}

#[test]
fn an_unwritable_journal_reports_its_path() {
    let dir = tempfile::tempdir().unwrap_or_else(|_| panic!("temp"));
    let path = dir.path().join("probe.txt");
    std::fs::write(&path, b"x").unwrap_or_else(|_| panic!("write"));
    let blocked = path.join("journal.jsonl");

    let error = match Journal::open(&blocked) {
        Ok(_) => panic!("a file cannot be a directory"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("journal"), "{error}");
}

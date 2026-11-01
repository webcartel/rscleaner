use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::Arc;

use rscleaner_app::{CleanRequest, Engine, EngineOptions, ScanRequest};
use rscleaner_exec::CleanOptions;
use rscleaner_i18n::Translator;
use rscleaner_model::{Config, Event, Platform, Risk, RuleId, RuleStatus};
use rscleaner_platform::{Elevation, Env, Fs, KnownFolders, Procs, SystemEnv, SystemInfo};
use rscleaner_rules::paths;
use rscleaner_rules::set::RuleSet;

use crate::cli::{CleanArgs, ScanArgs};

#[cfg(windows)]
use rscleaner_platform::{
    WindowsElevation, WindowsFs, WindowsKnownFolders, WindowsProcs, WindowsSystemInfo,
};

#[cfg(target_os = "linux")]
use rscleaner_platform::{LinuxElevation, LinuxFs, LinuxKnownFolders, LinuxProcs, LinuxSystemInfo};

pub struct Backends {
    pub fs: Arc<dyn Fs>,
    pub env: Arc<dyn Env>,
    pub known: Arc<dyn KnownFolders>,
    pub procs: Arc<dyn Procs>,
    pub elevation: Arc<dyn Elevation>,
    pub system: Arc<dyn SystemInfo>,
}

pub fn backends() -> Backends {
    #[cfg(windows)]
    {
        Backends {
            fs: Arc::new(WindowsFs::new()),
            env: Arc::new(SystemEnv),
            known: Arc::new(WindowsKnownFolders),
            procs: Arc::new(WindowsProcs),
            elevation: Arc::new(WindowsElevation),
            system: Arc::new(WindowsSystemInfo),
        }
    }
    #[cfg(target_os = "linux")]
    {
        Backends {
            fs: Arc::new(LinuxFs::new()),
            env: Arc::new(SystemEnv),
            known: Arc::new(LinuxKnownFolders),
            procs: Arc::new(LinuxProcs),
            elevation: Arc::new(LinuxElevation),
            system: Arc::new(LinuxSystemInfo),
        }
    }
}

pub fn open_engine(
    set: RuleSet,
    config: &Config,
    cli_rules_dir: Option<&std::path::Path>,
) -> Result<Engine, String> {
    let paths = paths::resolve(cli_rules_dir, false).map_err(|error| error.to_string())?;
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let backends = backends();
    Engine::open(EngineOptions {
        rules: set,
        config: config.clone(),
        cwd,
        fs: backends.fs,
        env: backends.env,
        known: backends.known,
        procs: backends.procs,
        elevation: backends.elevation,
        system: backends.system,
        journal_path: paths.logs.join(format!("{}.jsonl", day_stamp())),
    })
    .map_err(|error| error.to_string())
}

pub fn scan_command(
    engine: &Engine,
    args: &ScanArgs,
    json: bool,
    events: bool,
    translator: &Translator,
) -> i32 {
    let request = ScanRequest {
        profile: args.profile.clone().unwrap_or_default(),
        rule_masks: args.rules.clone(),
    };
    let job = engine.scan(request);
    let mut printed_events = 0;
    for event in job.events.iter() {
        if events || args.events {
            print_event(&event);
            printed_events += 1;
        }
    }
    let _ = printed_events;
    match job.result() {
        Ok(plan_id) => match engine.plan(plan_id) {
            Some(plan) if json => {
                match serde_json::to_string_pretty(&plan_document(&plan)) {
                    Ok(text) => println!("{text}"),
                    Err(error) => {
                        eprintln!("{error}");
                        return 1;
                    }
                }
                0
            }
            Some(plan) => {
                print_plan(&plan, translator);
                0
            }
            None => {
                eprintln!("the plan disappeared");
                1
            }
        },
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

pub fn clean_command(
    engine: &Engine,
    args: &CleanArgs,
    json: bool,
    events: bool,
    translator: &Translator,
) -> i32 {
    if args.on_reboot && cfg!(not(windows)) {
        eprintln!("{}", translator.t("cli.clean.on_reboot"));
        return 2;
    }

    let selected = engine.selected(&ScanRequest {
        profile: args.profile.clone().unwrap_or_default(),
        rule_masks: args.rules.clone(),
    });
    let dangerous: Vec<&RuleId> = selected
        .iter()
        .filter(|rule| rule.risk == Risk::Dangerous)
        .map(|rule| &rule.id)
        .collect();
    if !dangerous.is_empty() && !args.include_dangerous {
        eprintln!(
            "{}",
            translator.t_args(
                "cli.clean.dangerous",
                &[(
                    "rules",
                    &dangerous
                        .iter()
                        .map(|id| id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )]
            )
        );
        return 2;
    }
    if !dangerous.is_empty() && args.no_trash {
        eprintln!("{}", translator.t("cli.clean.no_trash_dangerous"));
        return 2;
    }

    let job = engine.scan(ScanRequest {
        profile: args.profile.clone().unwrap_or_default(),
        rule_masks: args.rules.clone(),
    });
    for event in job.events.iter() {
        if events || args.events {
            print_event(&event);
        }
    }
    let plan_id = match job.result() {
        Ok(plan_id) => plan_id,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let Some(plan) = engine.plan(plan_id) else {
        eprintln!("{}", translator.t("cli.error.plan"));
        return 1;
    };
    if !json {
        print_plan(&plan, translator);
    }
    if plan.is_empty() {
        if json {
            println!("{{}}");
        }
        return 0;
    }

    if !args.dry_run && !args.yes && !confirm(translator) {
        eprintln!("{}", translator.t("cli.clean.need_yes"));
        return 3;
    }

    let options = CleanOptions {
        dry_run: args.dry_run,
        use_trash: args.trash || (!args.no_trash && engine.settings().trash.default),
        on_reboot: args.on_reboot,
    };
    let job = engine.clean(CleanRequest {
        plan: plan_id,
        options,
    });
    for event in job.events.iter() {
        if events || args.events {
            print_event(&event);
        }
    }
    let output = match job.result() {
        Ok(output) => output,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    if let Some(path) = &args.report {
        if let Err(error) = write_report(path, &output.report) {
            eprintln!(
                "{}",
                translator.t_args("cli.error.report", &[("error", &error.to_string())])
            );
            return 1;
        }
    }
    if json {
        match serde_json::to_string_pretty(&output.report) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        }
    } else {
        println!(
            "{}",
            translator.t_args(
                "cli.clean.summary",
                &[
                    ("found", &output.summary.bytes_found.to_string()),
                    ("freed", &output.summary.bytes_freed.to_string()),
                    ("trashed", &output.summary.bytes_trashed.to_string()),
                    ("removed", &output.summary.removed.to_string()),
                    ("skipped", &output.summary.skipped.to_string()),
                    ("failed", &output.summary.failed.to_string()),
                ]
            )
        );
    }

    if output.cancelled {
        return 130;
    }
    if output.remaining > 0 { 4 } else { 0 }
}

fn confirm(translator: &Translator) -> bool {
    if !std::io::stdin().is_terminal() {
        return false;
    }
    print!("{}", translator.t("cli.clean.confirm"));
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn print_plan(plan: &rscleaner_scan::Plan, translator: &Translator) {
    for rule in &plan.rules {
        if rule.items.is_empty() && rule.skips.is_empty() {
            continue;
        }
        println!(
            "{}\t{}\t{}\t{} files\t{} bytes",
            rule.rule_id,
            rule.status_name(),
            rule.risk.as_str(),
            rule.totals.files,
            rule.totals.bytes_found
        );
        for skip in &rule.skips {
            println!("\tskip\t{}\t{}", skip.path.display, skip.reason.as_str());
        }
    }
    println!(
        "{}",
        translator.t_args(
            "cli.scan.total",
            &[
                ("files", &plan.totals.files.to_string()),
                ("bytes", &plan.totals.bytes_found.to_string()),
            ]
        )
    );
}

fn plan_document(plan: &rscleaner_scan::Plan) -> serde_json::Value {
    let rules: Vec<serde_json::Value> = plan
        .rules
        .iter()
        .map(|rule| {
            serde_json::json!({
                "id": rule.rule_id.as_str(),
                "status": status_name(rule.status),
                "risk": rule.risk.as_str(),
                "files": rule.totals.files,
                "dirs": rule.totals.dirs,
                "bytes": rule.totals.bytes_found,
                "items": rule.items.iter().map(|item| serde_json::json!({
                    "path": item.item.path,
                    "size": item.item.size_apparent,
                    "files": item.item.files,
                    "is_dir": item.item.is_dir,
                })).collect::<Vec<_>>(),
                "skips": rule.skips.iter().map(|skip| serde_json::json!({
                    "path": skip.path,
                    "reason": skip.reason.as_str(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::json!({
        "schema": 1,
        "platform": Platform::current().as_str(),
        "engine": env!("CARGO_PKG_VERSION"),
        "cancelled": plan.cancelled,
        "totals": {
            "files": plan.totals.files,
            "dirs": plan.totals.dirs,
            "bytes_found": plan.totals.bytes_found,
        },
        "rules": rules,
    })
}

fn print_event(event: &Event) {
    match serde_json::to_string(event) {
        Ok(text) => println!("{text}"),
        Err(error) => eprintln!("{error}"),
    }
}

fn write_report(path: &PathBuf, report: &rscleaner_report::Report) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(report).map_err(std::io::Error::other)?;
    std::fs::write(path, format!("{text}\n"))
}

fn status_name(status: RuleStatus) -> &'static str {
    match status {
        RuleStatus::Done => "done",
        RuleStatus::Skipped => "skipped",
        RuleStatus::Failed => "failed",
        RuleStatus::DryRun => "dry_run",
        RuleStatus::NotApplicable => "not_applicable",
    }
}

trait RulePlanDisplay {
    fn status_name(&self) -> &'static str;
}

impl RulePlanDisplay for rscleaner_scan::RulePlan {
    fn status_name(&self) -> &'static str {
        status_name(self.status)
    }
}

/// `YYYY-MM-DD` in UTC, for the daily journal file.
fn day_stamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Howard Hinnant's days-to-civil conversion.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let month = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn the_epoch_is_the_first_of_january_1970() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn a_known_day_matches_the_calendar() {
        assert_eq!(civil_from_days(20_697), (2026, 9, 1));
    }
}

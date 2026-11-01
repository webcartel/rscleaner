use rscleaner_guard::{Guard, GuardError, GuardReason, Intent, Remover};
use rscleaner_model::{
    CancelToken, ErrorCode, Event, ExecSummary, Mode, PathRepr, Platform, Risk, Rule, RuleId,
    RuleStatus, SkipReason, WarningCode,
};
use rscleaner_platform::{Elevation, Env, Fs, Procs};
use rscleaner_report::{
    FailEntry, Journal, JournalAction, JournalEntry, JournalResult, Report, RuleReport, SkipEntry,
};
use rscleaner_scan::RulePlan;

use crate::command::{self, CommandSpec};

#[derive(Clone, Copy, Debug, Default)]
pub struct CleanOptions {
    pub dry_run: bool,
    pub use_trash: bool,
    pub on_reboot: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error(transparent)]
    Journal(#[from] rscleaner_report::JournalError),
    #[error("{0}")]
    Fatal(String),
}

/// The result of a run: the report plus the numbers the CLI needs.
pub struct ExecOutput {
    pub report: Report,
    pub summary: ExecSummary,
    /// Items left behind by a skip or a failure.
    pub remaining: u64,
    pub cancelled: bool,
}

pub struct Executor<'a> {
    pub guard: &'a Guard,
    pub fs: &'a dyn Fs,
    pub env: &'a dyn Env,
    pub remover: &'a dyn Remover,
    pub procs: &'a dyn Procs,
    pub elevation: &'a dyn Elevation,
    pub journal: &'a mut Journal,
    pub cancel: CancelToken,
    pub options: CleanOptions,
    pub now_unix: i64,
    pending_reboot: Vec<PathRepr>,
}

impl<'a> Executor<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        guard: &'a Guard,
        fs: &'a dyn Fs,
        env: &'a dyn Env,
        remover: &'a dyn Remover,
        procs: &'a dyn Procs,
        elevation: &'a dyn Elevation,
        journal: &'a mut Journal,
        cancel: CancelToken,
        options: CleanOptions,
        now_unix: i64,
    ) -> Self {
        Self {
            guard,
            fs,
            env,
            remover,
            procs,
            elevation,
            journal,
            cancel,
            options,
            now_unix,
            pending_reboot: Vec::new(),
        }
    }

    pub fn run(
        &mut self,
        plan: &rscleaner_scan::Plan,
        rules: &[Rule],
        emit: &mut dyn FnMut(Event),
    ) -> Result<ExecOutput, ExecError> {
        let mut report = Report::new(
            Platform::current(),
            env!("CARGO_PKG_VERSION"),
            self.now_unix,
        );
        report.dry_run = self.options.dry_run;
        let mut summary = ExecSummary::default();
        let mut cancelled = false;

        for rule_plan in &plan.rules {
            let rule = rules.iter().find(|rule| rule.id == rule_plan.rule_id);
            let rule_report = self.run_rule(rule_plan, rule, emit, &mut summary, &mut cancelled)?;
            report.rules.push(rule_report);
        }

        report.finished_unix = self.now_unix;
        report.recompute();
        report.pending_reboot = self.pending_reboot.clone();
        summary.bytes_found = report.bytes_found;
        summary.bytes_freed = report.bytes_freed;
        summary.bytes_trashed = report.bytes_trashed;
        summary.pending_reboot = report.pending_reboot.len() as u64;
        let remaining = summary.skipped.saturating_add(summary.failed);

        Ok(ExecOutput {
            report,
            summary,
            remaining,
            cancelled,
        })
    }

    fn run_rule(
        &mut self,
        plan: &RulePlan,
        rule: Option<&Rule>,
        emit: &mut dyn FnMut(Event),
        summary: &mut ExecSummary,
        cancelled: &mut bool,
    ) -> Result<RuleReport, ExecError> {
        let mut report = RuleReport::new(plan.rule_id.clone(), plan.status, plan.risk);
        report.bytes_found = plan.totals.bytes_found;

        if let Some(reason) = self.rule_blocker(rule, &plan.rule_id, emit) {
            for item in &plan.items {
                self.record_skip(&mut report, summary, emit, &item.item.path, reason);
            }
            report.status = RuleStatus::Skipped;
            return Ok(report);
        }

        let dangerous = plan.risk == Risk::Dangerous;
        let trash = dangerous
            || rule.and_then(|rule| rule.trash).unwrap_or(false)
            || self.options.use_trash;

        if self.options.dry_run {
            report.removed = plan.items.len() as u64;
            summary.removed = summary.removed.saturating_add(report.removed);
            report.status = RuleStatus::DryRun;
            return Ok(report);
        }

        for item in &plan.items {
            if self.cancel.is_cancelled() {
                *cancelled = true;
                break;
            }
            let path = std::path::Path::new(&item.item.path.display);
            let verified =
                match self
                    .guard
                    .reverify_scan(path, Intent::Delete, &item.ctx, item.identity)
                {
                    Ok(verified) => verified,
                    Err(error) => {
                        self.record_skip(
                            &mut report,
                            summary,
                            emit,
                            &item.item.path,
                            error.skip_reason(),
                        );
                        continue;
                    }
                };

            let result = if trash {
                self.remover.trash(&verified)
            } else {
                self.remover.remove(&verified)
            };

            match result {
                Ok(()) => self.record_removed(
                    &mut report,
                    summary,
                    emit,
                    plan.rule_id.clone(),
                    &item.item.path,
                    item.item.size_apparent,
                    trash,
                )?,
                Err(error)
                    if error.reason == GuardReason::InUse
                        && self.can_defer(plan.risk, item.ctx.mode) =>
                {
                    match self.remover.remove_on_reboot(&verified) {
                        Ok(()) => {
                            report.removed = report.removed.saturating_add(1);
                            summary.removed = summary.removed.saturating_add(1);
                            self.pending_reboot.push(item.item.path.clone());
                            self.journal_record(
                                &plan.rule_id,
                                &item.item.path,
                                JournalAction::Reboot,
                                JournalResult::Ok,
                                0,
                            )?;
                            emit(Event::ItemRemoved {
                                rule_id: plan.rule_id.clone(),
                                path: item.item.path.clone(),
                                bytes: 0,
                                to_trash: false,
                            });
                        }
                        Err(defer_error) => self.record_failed(
                            &mut report,
                            summary,
                            emit,
                            &plan.rule_id,
                            &item.item.path,
                            &defer_error,
                        )?,
                    }
                }
                Err(error) => self.record_failed(
                    &mut report,
                    summary,
                    emit,
                    &plan.rule_id,
                    &item.item.path,
                    &error,
                )?,
            }
        }

        if !self.options.dry_run
            && let Some(rule) = rule
        {
            for target in &rule.targets {
                if let rscleaner_model::Target::Command {
                    cmd,
                    args,
                    timeout_s,
                } = target
                {
                    let spec = CommandSpec {
                        cmd: cmd.clone(),
                        args: args.clone(),
                        timeout_s: *timeout_s,
                    };
                    let label = PathRepr::text(cmd.clone());
                    match command::run(&spec, self.env, self.fs, &self.cancel) {
                        Ok(output) => {
                            self.journal_record(
                                &plan.rule_id,
                                &label,
                                JournalAction::Command,
                                JournalResult::Ok,
                                0,
                            )?;
                            emit(Event::Warning {
                                rule_id: Some(plan.rule_id.clone()),
                                code: WarningCode::CommandSucceeded,
                                detail: output.lines().take(3).collect::<Vec<_>>().join(" | "),
                            });
                        }
                        Err(error) => {
                            report.failed.push(FailEntry {
                                path: label.clone(),
                                code: ErrorCode::Command,
                                detail: error.to_string(),
                            });
                            summary.failed = summary.failed.saturating_add(1);
                            self.journal_record(
                                &plan.rule_id,
                                &label,
                                JournalAction::Fail,
                                JournalResult::Failed,
                                0,
                            )?;
                            emit(Event::ItemFailed {
                                rule_id: plan.rule_id.clone(),
                                path: label,
                                code: ErrorCode::Command,
                                detail: error.to_string(),
                            });
                        }
                    }
                }
            }
        }

        report.status = if *cancelled {
            RuleStatus::Skipped
        } else if !report.failed.is_empty() {
            RuleStatus::Failed
        } else if report.removed > 0 {
            RuleStatus::Done
        } else if !plan.items.is_empty() {
            RuleStatus::Skipped
        } else if rule.is_some_and(command::has_command_targets) && !self.options.dry_run {
            RuleStatus::Done
        } else {
            RuleStatus::Skipped
        };
        Ok(report)
    }

    fn rule_blocker(
        &self,
        rule: Option<&Rule>,
        rule_id: &RuleId,
        emit: &mut dyn FnMut(Event),
    ) -> Option<SkipReason> {
        let Some(rule) = rule else {
            return Some(SkipReason::NotApplicable);
        };
        if !rule.requires_closed.is_empty()
            && let Ok(running) = self.procs.running(&rule.requires_closed)
            && !running.is_empty()
        {
            emit(Event::Warning {
                rule_id: Some(rule_id.clone()),
                code: WarningCode::ProcessRunning,
                detail: format!("running: {}", running.join(", ")),
            });
            return Some(SkipReason::AppRunning);
        }
        if rule.requires_admin && !self.elevation.is_elevated() {
            emit(Event::Warning {
                rule_id: Some(rule_id.clone()),
                code: WarningCode::NotElevated,
                detail: "the rule needs administrator rights".into(),
            });
            return Some(SkipReason::NeedsAdmin);
        }
        None
    }

    fn can_defer(&self, risk: Risk, mode: Mode) -> bool {
        cfg!(windows) && self.options.on_reboot && risk != Risk::Dangerous && mode != Mode::Contents
    }

    #[allow(clippy::too_many_arguments)]
    fn record_removed(
        &mut self,
        report: &mut RuleReport,
        summary: &mut ExecSummary,
        emit: &mut dyn FnMut(Event),
        rule_id: RuleId,
        path: &PathRepr,
        bytes: u64,
        trash: bool,
    ) -> Result<(), ExecError> {
        report.removed = report.removed.saturating_add(1);
        summary.removed = summary.removed.saturating_add(1);
        if trash {
            report.bytes_trashed = report.bytes_trashed.saturating_add(bytes);
        } else {
            report.bytes_freed = report.bytes_freed.saturating_add(bytes);
        }
        self.journal_record(
            &rule_id,
            path,
            if trash {
                JournalAction::Trash
            } else {
                JournalAction::Remove
            },
            JournalResult::Ok,
            bytes,
        )?;
        emit(Event::ItemRemoved {
            rule_id,
            path: path.clone(),
            bytes,
            to_trash: trash,
        });
        Ok(())
    }

    fn record_skip(
        &mut self,
        report: &mut RuleReport,
        summary: &mut ExecSummary,
        emit: &mut dyn FnMut(Event),
        path: &PathRepr,
        reason: SkipReason,
    ) {
        report.skipped.push(SkipEntry {
            path: path.clone(),
            reason,
        });
        summary.skipped = summary.skipped.saturating_add(1);
        let rule_id = report.id.clone();
        let _ = self.journal_record(
            &rule_id,
            path,
            JournalAction::Skip,
            JournalResult::Skipped,
            0,
        );
        emit(Event::ItemSkipped {
            rule_id,
            path: path.clone(),
            reason,
        });
    }

    fn record_failed(
        &mut self,
        report: &mut RuleReport,
        summary: &mut ExecSummary,
        emit: &mut dyn FnMut(Event),
        rule_id: &RuleId,
        path: &PathRepr,
        error: &GuardError,
    ) -> Result<(), ExecError> {
        let code = error_code(error);
        report.failed.push(FailEntry {
            path: path.clone(),
            code,
            detail: error.to_string(),
        });
        summary.failed = summary.failed.saturating_add(1);
        self.journal_record(rule_id, path, JournalAction::Fail, JournalResult::Failed, 0)?;
        emit(Event::ItemFailed {
            rule_id: rule_id.clone(),
            path: path.clone(),
            code,
            detail: error.to_string(),
        });
        Ok(())
    }

    fn journal_record(
        &mut self,
        rule: &RuleId,
        path: &PathRepr,
        action: JournalAction,
        result: JournalResult,
        bytes: u64,
    ) -> Result<(), ExecError> {
        let entry = JournalEntry {
            time_unix: self.now_unix,
            rule: rule.clone(),
            path: path.clone(),
            action,
            bytes,
            result,
        };
        self.journal.append(&entry)?;
        Ok(())
    }
}

fn error_code(error: &GuardError) -> ErrorCode {
    match error.reason {
        GuardReason::InUse => ErrorCode::InUse,
        _ => ErrorCode::Io,
    }
}

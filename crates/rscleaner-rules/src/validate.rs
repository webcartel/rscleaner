use std::collections::HashSet;

use rscleaner_model::{ErrorCode, Platform, Risk, Rule, RuleError, RuleFile, Target};
use rscleaner_platform::{Env, KnownFolders};

use crate::glob::CompiledGlob;
use crate::vars::expand_vars;

#[derive(Clone, Debug)]
pub struct ValidationOptions {
    pub allow_user_commands: bool,
    pub allow_registry: bool,
    pub platform: Platform,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            allow_user_commands: false,
            allow_registry: false,
            platform: Platform::Windows,
        }
    }
}

pub fn validate_file(
    file: &RuleFile,
    env: &dyn Env,
    known: &dyn KnownFolders,
    opts: &ValidationOptions,
) -> Vec<RuleError> {
    validate_file_with_text(file, "", env, known, opts)
}

pub fn validate_file_with_text(
    file: &RuleFile,
    text: &str,
    env: &dyn Env,
    known: &dyn KnownFolders,
    opts: &ValidationOptions,
) -> Vec<RuleError> {
    let mut errors = Vec::new();
    let mut ids = HashSet::new();

    for (index, rule) in file.rules.iter().enumerate() {
        let line = rule_line(text, index, rule.id.as_str());
        let mut add = |code: ErrorCode, message: String| {
            errors.push(RuleError::new("", line, 1, code, message));
        };

        if !valid_id(rule.id.as_str()) {
            add(
                ErrorCode::IdPattern,
                format!("invalid rule id: {}", rule.id),
            );
        }

        if !ids.insert(rule.id.as_str().to_string()) {
            add(
                ErrorCode::DuplicateId,
                format!("duplicate rule id: {}", rule.id),
            );
        }

        if rule.targets.is_empty() {
            add(ErrorCode::EmptyTargets, "rule has no targets".into());
        }

        if rule.os_min_build.is_some() && opts.platform != Platform::Windows {
            add(
                ErrorCode::PlatformWhen,
                "os_min_build is only valid on Windows".into(),
            );
        }

        if let Some(when) = &rule.when {
            if !when.distro.is_empty() && opts.platform != Platform::Linux {
                add(
                    ErrorCode::PlatformWhen,
                    "when.distro is only valid on Linux".into(),
                );
            }
        }

        for target in &rule.targets {
            match target {
                Target::Path {
                    path,
                    include,
                    exclude,
                    ..
                } => {
                    if path.is_empty() {
                        add(ErrorCode::Schema, "path target has an empty path".into());
                    } else {
                        match expand_vars(path, env, known) {
                            Ok(result) => {
                                if result.missing_known.is_empty() {
                                    if let Err(error) = CompiledGlob::compile(&result.path) {
                                        add(ErrorCode::BadGlob, error.to_string());
                                    }
                                }
                            }
                            Err(error) => add(ErrorCode::UnknownVar, error.to_string()),
                        }
                    }
                    for mask in include.iter().chain(exclude.iter()) {
                        if mask.is_empty() {
                            add(ErrorCode::EmptyMask, "empty include/exclude mask".into());
                        } else if let Err(error) = CompiledGlob::compile(mask) {
                            add(ErrorCode::BadGlob, error.to_string());
                        }
                    }
                }
                Target::Find {
                    roots,
                    name,
                    max_depth,
                    ..
                } => {
                    if name.is_empty() {
                        add(ErrorCode::Schema, "find target has an empty name".into());
                    }
                    if *max_depth == Some(0) {
                        add(ErrorCode::Schema, "find max_depth must be positive".into());
                    }
                    for root in roots {
                        if let Err(error) = expand_vars(root, env, known) {
                            add(ErrorCode::UnknownVar, error.to_string());
                        }
                    }
                }
                Target::Command { cmd, timeout_s, .. } => {
                    if cmd.is_empty() {
                        add(ErrorCode::Schema, "command target has an empty cmd".into());
                    }
                    if *timeout_s == Some(0) {
                        add(ErrorCode::Schema, "command timeout must be positive".into());
                    }
                    if !opts.allow_user_commands {
                        add(
                            ErrorCode::CommandDisabled,
                            "user commands are disabled".into(),
                        );
                    }
                }
                Target::Registry(_) => {
                    if !opts.allow_registry {
                        add(
                            ErrorCode::RegistryUser,
                            "registry targets are built-in only".into(),
                        );
                    }
                    if opts.platform != Platform::Windows {
                        add(
                            ErrorCode::PlatformWhen,
                            "registry targets are Windows-only".into(),
                        );
                    }
                }
            }
        }

        let inferred = inferred_risk(rule, env, known);
        if rule.risk < inferred {
            add(
                ErrorCode::RiskUnderstated,
                format!(
                    "declared risk {} is below inferred risk {}",
                    rule.risk.as_str(),
                    inferred.as_str()
                ),
            );
        }
    }

    errors
}

fn valid_id(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().enumerate().all(|(index, part)| {
        if part.is_empty() {
            return false;
        }
        if index == 0 {
            part.chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        } else {
            part.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
            })
        }
    })
}

fn rule_line(text: &str, index: usize, id: &str) -> usize {
    if !text.is_empty() {
        let needle_a = format!("- id: {id}");
        let needle_b = format!("id: {id}");
        if let Some(line) = text
            .lines()
            .position(|line| line.contains(&needle_a) || line.contains(&needle_b))
        {
            return line + 1;
        }
    }
    index + 1
}

fn inferred_risk(rule: &Rule, env: &dyn Env, known: &dyn KnownFolders) -> Risk {
    let mut risk = Risk::Safe;
    for target in &rule.targets {
        match target {
            Target::Find { .. } | Target::Command { .. } => risk = risk.max(Risk::Moderate),
            Target::Path { path, .. } => {
                let expanded = expand_vars(path, env, known)
                    .map(|value| value.path)
                    .unwrap_or_else(|_| path.clone());
                if is_user_data(&expanded) {
                    risk = risk.max(Risk::Dangerous);
                }
            }
            Target::Registry(_) => {}
        }
    }
    if rule.category.eq_ignore_ascii_case("user") {
        risk = risk.max(Risk::Dangerous);
    }
    risk
}

fn is_user_data(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("${known:")
        || lower.contains("userprofile")
        || lower.contains("/users/")
        || lower.contains("/documents")
        || lower.contains("/downloads")
        || lower.contains("/desktop")
}

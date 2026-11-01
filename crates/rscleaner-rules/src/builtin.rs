use std::path::Path;

use rscleaner_model::{Platform, Rule, RuleError, RuleFile};
use rscleaner_platform::{EmptyKnownFolders, SystemEnv};

use crate::error::yaml_error;
use crate::set::{RuleSet, RuleSourceSet, merge_sources};
use crate::validate::{ValidationOptions, validate_file_with_text};

const WINDOWS: &[(&str, &str)] = &[
    (
        "apps/discord.yaml",
        include_str!("../../../rules/windows/apps/discord.yaml"),
    ),
    (
        "apps/notion.yaml",
        include_str!("../../../rules/windows/apps/notion.yaml"),
    ),
    (
        "apps/obsidian.yaml",
        include_str!("../../../rules/windows/apps/obsidian.yaml"),
    ),
    (
        "apps/slack.yaml",
        include_str!("../../../rules/windows/apps/slack.yaml"),
    ),
    (
        "apps/steam.yaml",
        include_str!("../../../rules/windows/apps/steam.yaml"),
    ),
    (
        "apps/telegram.yaml",
        include_str!("../../../rules/windows/apps/telegram.yaml"),
    ),
    (
        "apps/vscode.yaml",
        include_str!("../../../rules/windows/apps/vscode.yaml"),
    ),
    (
        "browsers/chrome.yaml",
        include_str!("../../../rules/windows/browsers/chrome.yaml"),
    ),
    (
        "browsers/edge.yaml",
        include_str!("../../../rules/windows/browsers/edge.yaml"),
    ),
    (
        "browsers/firefox.yaml",
        include_str!("../../../rules/windows/browsers/firefox.yaml"),
    ),
    (
        "dev/docker.yaml",
        include_str!("../../../rules/windows/dev/docker.yaml"),
    ),
    (
        "dev/go-build.yaml",
        include_str!("../../../rules/windows/dev/go-build.yaml"),
    ),
    (
        "dev/gradle.yaml",
        include_str!("../../../rules/windows/dev/gradle.yaml"),
    ),
    (
        "dev/npm.yaml",
        include_str!("../../../rules/windows/dev/npm.yaml"),
    ),
    (
        "dev/nuget.yaml",
        include_str!("../../../rules/windows/dev/nuget.yaml"),
    ),
    (
        "dev/pip.yaml",
        include_str!("../../../rules/windows/dev/pip.yaml"),
    ),
    (
        "dev/pnpm.yaml",
        include_str!("../../../rules/windows/dev/pnpm.yaml"),
    ),
    (
        "dev/yarn.yaml",
        include_str!("../../../rules/windows/dev/yarn.yaml"),
    ),
    (
        "downloads/old-installers.yaml",
        include_str!("../../../rules/windows/downloads/old-installers.yaml"),
    ),
    (
        "system/cbs-logs.yaml",
        include_str!("../../../rules/windows/system/cbs-logs.yaml"),
    ),
    (
        "system/component-cleanup.yaml",
        include_str!("../../../rules/windows/system/component-cleanup.yaml"),
    ),
    (
        "system/crash-dumps.yaml",
        include_str!("../../../rules/windows/system/crash-dumps.yaml"),
    ),
    (
        "system/delivery-optimization.yaml",
        include_str!("../../../rules/windows/system/delivery-optimization.yaml"),
    ),
    (
        "system/minidumps.yaml",
        include_str!("../../../rules/windows/system/minidumps.yaml"),
    ),
    (
        "system/recycle-bin.yaml",
        include_str!("../../../rules/windows/system/recycle-bin.yaml"),
    ),
    (
        "system/temp.yaml",
        include_str!("../../../rules/windows/system/temp.yaml"),
    ),
    (
        "system/thumbnail-cache.yaml",
        include_str!("../../../rules/windows/system/thumbnail-cache.yaml"),
    ),
    (
        "system/wer-machine.yaml",
        include_str!("../../../rules/windows/system/wer-machine.yaml"),
    ),
    (
        "system/wer-user.yaml",
        include_str!("../../../rules/windows/system/wer-user.yaml"),
    ),
    (
        "system/windows-old.yaml",
        include_str!("../../../rules/windows/system/windows-old.yaml"),
    ),
    (
        "system/windows-temp.yaml",
        include_str!("../../../rules/windows/system/windows-temp.yaml"),
    ),
    (
        "system/windows-update-cache.yaml",
        include_str!("../../../rules/windows/system/windows-update-cache.yaml"),
    ),
];

const LINUX: &[(&str, &str)] = &[
    (
        "linux/apt-clean.yaml",
        include_str!("../../../rules/linux/apt-clean.yaml"),
    ),
    (
        "linux/tmp.yaml",
        include_str!("../../../rules/linux/tmp.yaml"),
    ),
    (
        "linux/logs.yaml",
        include_str!("../../../rules/linux/logs.yaml"),
    ),
    (
        "linux/user-caches.yaml",
        include_str!("../../../rules/linux/user-caches.yaml"),
    ),
    (
        "linux/trash.yaml",
        include_str!("../../../rules/linux/trash.yaml"),
    ),
];

pub fn load_builtin() -> Result<RuleSet, Vec<RuleError>> {
    let env = SystemEnv;
    let known = EmptyKnownFolders;
    let mut errors = Vec::new();
    let mut sources = Vec::new();

    for (name, text) in WINDOWS {
        match load_text(name, text, Platform::Windows, &env, &known) {
            Ok(rules) => sources.push(RuleSourceSet {
                name: format!("builtin:{name}"),
                rules,
            }),
            Err(mut found) => errors.append(&mut found),
        }
    }

    for (name, text) in LINUX {
        match load_text(name, text, Platform::Linux, &env, &known) {
            Ok(rules) => sources.push(RuleSourceSet {
                name: format!("builtin:{name}"),
                rules,
            }),
            Err(mut found) => errors.append(&mut found),
        }
    }

    if errors.is_empty() {
        Ok(merge_sources(sources))
    } else {
        Err(errors)
    }
}

fn load_text(
    name: &str,
    text: &str,
    platform: Platform,
    env: &SystemEnv,
    known: &EmptyKnownFolders,
) -> Result<Vec<Rule>, Vec<RuleError>> {
    let file: RuleFile =
        serde_norway::from_str(text).map_err(|error| vec![yaml_error(Path::new(name), error)])?;
    let opts = ValidationOptions {
        allow_user_commands: true,
        allow_registry: true,
        platform,
    };
    let mut errors = validate_file_with_text(&file, text, env, known, &opts);
    if errors.is_empty() {
        Ok(file.rules)
    } else {
        for error in &mut errors {
            error.file = name.to_string();
        }
        Err(errors)
    }
}

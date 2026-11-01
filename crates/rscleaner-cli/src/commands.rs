use std::path::Path;
use std::str::FromStr;

use clap::CommandFactory;
use rscleaner_i18n::{Language, Translator};
use rscleaner_model::{Config, Platform, Rule, RuleError};
use rscleaner_platform::{EmptyKnownFolders, SystemEnv};
use rscleaner_rules::builtin::load_builtin;
use rscleaner_rules::config_file;
use rscleaner_rules::load::load_file;
use rscleaner_rules::paths;
use rscleaner_rules::set::{RuleSet as Rules, select};
use rscleaner_rules::validate::{ValidationOptions, validate_file};

use crate::clean::{clean_command, open_engine, scan_command};
use crate::cli::{Cli, Command, ConfigCommand, RulesCommand};

pub fn run(cli: Cli) -> i32 {
    let config = match load_config(&cli) {
        Ok(config) => config,
        Err(errors) => {
            print_errors(&errors);
            return 2;
        }
    };

    let language = match resolve_language(&cli, &config) {
        Some(language) => language,
        None => {
            eprintln!(
                "unsupported language: {}",
                cli.lang.as_deref().unwrap_or("")
            );
            return 2;
        }
    };
    let translator = Translator::new(language);

    let command = cli.command.clone();
    match command {
        None => {
            if let Err(error) = Cli::command().print_help() {
                eprintln!("{error}");
                return 1;
            }
            println!();
            0
        }
        Some(Command::Version) => {
            println!(
                "{}",
                translator.t_args(
                    "cli.version",
                    &[
                        ("version", env!("CARGO_PKG_VERSION")),
                        ("platform", std::env::consts::OS),
                    ]
                )
            );
            0
        }
        Some(Command::List {
            profile,
            category,
            platform,
        }) => list_rules(
            &cli,
            &config,
            language,
            profile.as_deref().unwrap_or("default"),
            category.as_deref(),
            platform.as_deref(),
        ),
        Some(Command::Scan(args)) => scan_command_flow(&cli, &config, &args, &translator),
        Some(Command::Clean(args)) => clean_command_flow(&cli, &config, &args, &translator),
        Some(Command::Rules { command }) => match command {
            RulesCommand::Validate { paths } => validate_paths(&config, &paths),
            RulesCommand::Show { id } => show_rule(&cli, &config, &id),
        },
        Some(Command::Config {
            command: ConfigCommand::Path,
        }) => show_paths(cli.json),
    }
}

fn scan_command_flow(
    cli: &Cli,
    config: &Config,
    args: &crate::cli::ScanArgs,
    translator: &Translator,
) -> i32 {
    let set = match load_rule_set(cli, config) {
        Ok(set) => set,
        Err(errors) => {
            print_errors(&errors);
            return 2;
        }
    };
    let engine = match open_engine(set, config, cli.rules_dir.as_deref()) {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    scan_command(&engine, args, cli.json, false, translator)
}

fn clean_command_flow(
    cli: &Cli,
    config: &Config,
    args: &crate::cli::CleanArgs,
    translator: &Translator,
) -> i32 {
    let set = match load_rule_set(cli, config) {
        Ok(set) => set,
        Err(errors) => {
            print_errors(&errors);
            return 2;
        }
    };
    let engine = match open_engine(set, config, cli.rules_dir.as_deref()) {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    clean_command(&engine, args, cli.json, false, translator)
}

fn load_config(cli: &Cli) -> Result<Config, Vec<RuleError>> {
    if let Some(path) = &cli.config {
        return config_file::load(path);
    }
    let paths = match paths::resolve(None, false) {
        Ok(paths) => paths,
        Err(error) => {
            return Err(vec![RuleError::new(
                "",
                1,
                1,
                rscleaner_model::ErrorCode::Schema,
                error.to_string(),
            )]);
        }
    };
    if paths.config.exists() {
        config_file::load(&paths.config)
    } else {
        Ok(Config::default())
    }
}

fn resolve_language(cli: &Cli, config: &Config) -> Option<Language> {
    if let Some(value) = cli.lang.as_deref() {
        return Language::from_str(value).ok();
    }
    if let Some(value) = config.language.as_deref() {
        if let Ok(language) = Language::from_str(value) {
            return Some(language);
        }
    }
    Some(Language::detect(&|name| std::env::var(name).ok()))
}

fn list_rules(
    cli: &Cli,
    config: &Config,
    language: Language,
    profile: &str,
    category: Option<&str>,
    platform: Option<&str>,
) -> i32 {
    let set = match load_rule_set(cli, config) {
        Ok(set) => set,
        Err(errors) => {
            print_errors(&errors);
            return 2;
        }
    };
    let selected = select(&set, config, profile);
    let platform = platform.and_then(|value| match value {
        "windows" => Some(Platform::Windows),
        "linux" => Some(Platform::Linux),
        _ => None,
    });
    let rules: Vec<&Rule> = selected
        .rules
        .iter()
        .filter(|rule| category.is_none_or(|value| rule.category == value))
        .filter(|rule| platform.is_none_or(|value| rule.platforms.contains(&value)))
        .collect();

    if cli.json {
        let values: Vec<_> = rules
            .iter()
            .map(|rule| {
                serde_json::json!({
                    "id": rule.id.as_str(),
                    "name": rule.name.resolve("en"),
                    "category": rule.category,
                    "risk": rule.risk.as_str(),
                    "platforms": rule.platforms.iter().map(|value| value.as_str()).collect::<Vec<_>>(),
                })
            })
            .collect();
        match serde_json::to_string_pretty(&values) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        }
    } else {
        for rule in rules {
            println!(
                "{}\t{}\t{}\t{}",
                rule.id,
                rule.name.resolve(language.code()),
                rule.risk.as_str(),
                rule.category
            );
        }
    }
    0
}

fn validate_paths(config: &Config, paths: &[std::path::PathBuf]) -> i32 {
    let env = SystemEnv;
    let known = EmptyKnownFolders;
    let opts = ValidationOptions {
        allow_user_commands: config.allow_user_commands,
        allow_registry: false,
        platform: current_platform(),
    };
    let mut errors = Vec::new();

    for path in paths {
        if path.is_dir() {
            let mut files: Vec<_> = match std::fs::read_dir(path) {
                Ok(entries) => entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|entry| {
                        entry.extension().and_then(|value| value.to_str()) == Some("yaml")
                    })
                    .collect(),
                Err(error) => {
                    eprintln!("{}: {error}", path.display());
                    return 2;
                }
            };
            files.sort();
            for file in files {
                collect_file_errors(&file, &env, &known, &opts, &mut errors);
            }
        } else {
            collect_file_errors(path, &env, &known, &opts, &mut errors);
        }
    }

    if errors.is_empty() {
        0
    } else {
        print_errors(&errors);
        2
    }
}

fn collect_file_errors(
    path: &Path,
    env: &SystemEnv,
    known: &EmptyKnownFolders,
    opts: &ValidationOptions,
    errors: &mut Vec<RuleError>,
) {
    match load_file(path) {
        Ok(file) => {
            let mut found = validate_file(&file, env, known, opts);
            for error in &mut found {
                error.file = path.display().to_string();
            }
            errors.append(&mut found);
        }
        Err(mut found) => errors.append(&mut found),
    }
}

fn show_rule(cli: &Cli, config: &Config, id: &str) -> i32 {
    let set = match load_rule_set(cli, config) {
        Ok(set) => set,
        Err(errors) => {
            print_errors(&errors);
            return 2;
        }
    };
    let Some(rule) = set.rules.get(id) else {
        eprintln!("unknown rule: {id}");
        return 2;
    };
    match serde_norway::to_string(rule) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn show_paths(json: bool) -> i32 {
    let paths = match paths::resolve(None, false) {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    if json {
        let value = serde_json::json!({
            "config": paths.config,
            "rules": paths.rules,
            "logs": paths.logs,
            "cache": paths.cache,
            "registry_backups": paths.registry_backups,
        });
        println!("{value}");
    } else {
        println!("config={}", paths.config.display());
        println!("rules={}", paths.rules.display());
        println!("logs={}", paths.logs.display());
        println!("cache={}", paths.cache.display());
        println!("registry_backups={}", paths.registry_backups.display());
    }
    0
}

fn load_rule_set(cli: &Cli, config: &Config) -> Result<Rules, Vec<RuleError>> {
    let mut set = load_builtin()?;
    let Some(directory) = cli.rules_dir.as_deref() else {
        return Ok(set);
    };
    if !directory.exists() {
        return Err(vec![RuleError::new(
            directory.display().to_string(),
            1,
            1,
            rscleaner_model::ErrorCode::Schema,
            "rules directory does not exist",
        )]);
    }
    let mut files: Vec<_> = std::fs::read_dir(directory)
        .map_err(|error| {
            vec![RuleError::new(
                directory.display().to_string(),
                1,
                1,
                rscleaner_model::ErrorCode::Yaml,
                error.to_string(),
            )]
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|entry| entry.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect();
    files.sort();

    let env = SystemEnv;
    let known = EmptyKnownFolders;
    let opts = ValidationOptions {
        allow_user_commands: config.allow_user_commands,
        allow_registry: false,
        platform: current_platform(),
    };
    for file in files {
        let loaded = load_file(&file)?;
        let mut errors = validate_file(&loaded, &env, &known, &opts);
        if !errors.is_empty() {
            for error in &mut errors {
                error.file = file.display().to_string();
            }
            return Err(errors);
        }
        for rule in loaded.rules {
            set.rules.insert(rule.id.clone(), rule);
        }
    }
    Ok(set)
}

fn current_platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

fn print_errors(errors: &[RuleError]) {
    for error in errors {
        eprintln!("{error}");
    }
}

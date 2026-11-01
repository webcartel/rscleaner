use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rscleaner", version, about = "RSCleaner")]
pub struct Cli {
    #[arg(long, global = true)]
    pub lang: Option<String>,
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    #[arg(long, global = true)]
    pub rules_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long = "no-color", global = true)]
    pub no_color: bool,
    #[arg(long, global = true)]
    pub ascii: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Version,
    Scan(ScanArgs),
    Clean(CleanArgs),
    List {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        platform: Option<String>,
    },
    Rules {
        #[command(subcommand)]
        command: RulesCommand,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Clone, clap::Args)]
pub struct ScanArgs {
    #[arg(long)]
    pub profile: Option<String>,
    #[arg(long = "rule")]
    pub rules: Vec<String>,
    #[arg(long = "events")]
    pub events: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct CleanArgs {
    #[arg(long)]
    pub profile: Option<String>,
    #[arg(long = "rule")]
    pub rules: Vec<String>,
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    #[arg(long)]
    pub yes: bool,
    #[arg(long = "trash", conflicts_with = "no_trash")]
    pub trash: bool,
    #[arg(long = "no-trash")]
    pub no_trash: bool,
    #[arg(long = "include-dangerous")]
    pub include_dangerous: bool,
    #[arg(long = "on-reboot")]
    pub on_reboot: bool,
    #[arg(long)]
    pub report: Option<PathBuf>,
    #[arg(long = "events")]
    pub events: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum RulesCommand {
    Validate {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    Show {
        id: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommand {
    Path,
}

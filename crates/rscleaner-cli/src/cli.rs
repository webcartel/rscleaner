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

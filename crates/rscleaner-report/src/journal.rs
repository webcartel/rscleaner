use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use rscleaner_model::{PathRepr, RuleId};

/// One appended line of the journal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JournalEntry {
    /// Time in seconds since the Unix epoch.
    pub time_unix: i64,
    pub rule: RuleId,
    pub path: PathRepr,
    pub action: JournalAction,
    pub bytes: u64,
    pub result: JournalResult,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JournalAction {
    Remove,
    Trash,
    Reboot,
    Command,
    Skip,
    Fail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JournalResult {
    Ok,
    Skipped,
    Failed,
}

#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("cannot open the journal at {path}: {source}")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot write to the journal at {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// An append-only jsonl journal. A run opens it before the first removal.
pub struct Journal {
    path: PathBuf,
    writer: BufWriter<File>,
}

impl Journal {
    pub fn open(path: &Path) -> Result<Self, JournalError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| JournalError::Open {
                path: path.to_path_buf(),
                source,
            })?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|source| JournalError::Open {
                path: path.to_path_buf(),
                source,
            })?;
        Ok(Self {
            path: path.to_path_buf(),
            writer: BufWriter::new(file),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&mut self, entry: &JournalEntry) -> Result<(), JournalError> {
        let line = serde_json::to_string(entry).map_err(|source| JournalError::Write {
            path: self.path.clone(),
            source: std::io::Error::other(source),
        })?;
        writeln!(self.writer, "{line}").map_err(|source| JournalError::Write {
            path: self.path.clone(),
            source,
        })?;
        self.writer.flush().map_err(|source| JournalError::Write {
            path: self.path.clone(),
            source,
        })
    }
}

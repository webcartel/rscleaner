use std::path::{Path, PathBuf};
use std::sync::Arc;

use rscleaner_platform::{FileIdentity, Fs, VolumeKind};

use crate::error::{GuardError, GuardReason};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Delete,
    Trash,
}

#[derive(Clone, Debug)]
pub struct VerifiedPath {
    pub(crate) given: PathBuf,
    pub(crate) final_path: PathBuf,
    pub(crate) identity: FileIdentity,
    pub(crate) volume: VolumeKind,
    pub(crate) intent: Intent,
    pub(crate) is_dir: bool,
}

impl VerifiedPath {
    pub fn given(&self) -> &Path {
        &self.given
    }

    pub fn final_path(&self) -> &Path {
        &self.final_path
    }

    pub fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub fn volume(&self) -> VolumeKind {
        self.volume
    }

    pub fn intent(&self) -> Intent {
        self.intent
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
}

pub trait Remover {
    fn remove(&self, path: &VerifiedPath) -> Result<(), GuardError>;
    fn trash(&self, path: &VerifiedPath) -> Result<(), GuardError>;
    fn remove_on_reboot(&self, path: &VerifiedPath) -> Result<(), GuardError>;
}

#[derive(Clone)]
pub struct FsRemover {
    fs: Arc<dyn Fs>,
}

impl FsRemover {
    pub fn new(fs: Arc<dyn Fs>) -> Self {
        Self { fs }
    }
}

impl Remover for FsRemover {
    fn remove(&self, path: &VerifiedPath) -> Result<(), GuardError> {
        let first = if path.is_dir {
            self.fs.remove_dir_raw(path.given())
        } else {
            self.fs.remove_file_raw(path.given())
        };
        let result = match first {
            Ok(()) => Ok(()),
            Err(first_error) => {
                let second = if path.is_dir {
                    self.fs.remove_file_raw(path.given())
                } else {
                    self.fs.remove_dir_raw(path.given())
                };
                second.map_err(|_| first_error)
            }
        };
        result.map_err(|error| {
            GuardError::new(
                GuardReason::Unsupported,
                format!("cannot remove {}: {error}", path.given().display()),
            )
        })
    }

    fn trash(&self, path: &VerifiedPath) -> Result<(), GuardError> {
        self.fs.trash_raw(path.given()).map_err(|error| {
            GuardError::new(
                GuardReason::Unsupported,
                format!("cannot move {} to trash: {error}", path.given().display()),
            )
        })
    }

    fn remove_on_reboot(&self, path: &VerifiedPath) -> Result<(), GuardError> {
        self.fs.remove_on_reboot_raw(path.given()).map_err(|error| {
            GuardError::new(
                GuardReason::Unsupported,
                format!("cannot defer {}: {error}", path.given().display()),
            )
        })
    }
}

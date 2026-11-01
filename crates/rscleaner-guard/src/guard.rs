use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rscleaner_model::Mode;
use rscleaner_platform::{Env, FileIdentity, Fs, KnownFolders, PlatformKind, VolumeKind};

use crate::error::{GuardError, GuardReason};
use crate::normalize::{NormalizedPath, normalize};
use crate::protected::{ProtectionTable, browser_profile_allows};
use crate::verified::{Intent, VerifiedPath};

#[derive(Clone, Debug)]
pub struct RuleCtx {
    pub mode: Mode,
    pub include: Vec<String>,
    pub min_age_days: Option<u32>,
}

impl Default for RuleCtx {
    fn default() -> Self {
        Self {
            mode: Mode::Contents,
            include: Vec::new(),
            min_age_days: None,
        }
    }
}

impl RuleCtx {
    fn filtered(&self) -> bool {
        self.mode == Mode::Files && !self.include.is_empty() && self.min_age_days.unwrap_or(0) > 0
    }
}

pub struct Guard {
    fs: Arc<dyn Fs>,
    env: Arc<dyn Env>,
    known: Arc<dyn KnownFolders>,
    cwd: PathBuf,
    table: ProtectionTable,
}

impl Guard {
    pub fn new(
        fs: Arc<dyn Fs>,
        env: Arc<dyn Env>,
        known: Arc<dyn KnownFolders>,
        cwd: PathBuf,
        extra: &[String],
    ) -> Result<Self, GuardError> {
        let table =
            ProtectionTable::new(env.as_ref(), known.as_ref(), PlatformKind::current(), extra);
        Ok(Self {
            fs,
            env,
            known,
            cwd,
            table,
        })
    }

    pub fn verify(
        &self,
        path: &Path,
        intent: Intent,
        ctx: &RuleCtx,
    ) -> Result<VerifiedPath, GuardError> {
        let normalized = normalize(
            self.fs.as_ref(),
            self.env.as_ref(),
            self.known.as_ref(),
            &self.cwd,
            path,
        )?;
        let meta = match self.fs.metadata(&normalized.final_path) {
            Ok(meta) => meta,
            Err(error) => return Err(io_error(error)),
        };
        if meta.volume != VolumeKind::Fixed {
            return Err(GuardError::new(
                GuardReason::NotFixedVolume,
                format!("volume is not fixed: {}", normalized.given.display()),
            ));
        }
        if meta.is_symlink && ctx.mode == Mode::Contents {
            return Err(GuardError::new(
                GuardReason::Protected,
                format!(
                    "cannot clean through a link: {}",
                    normalized.given.display()
                ),
            ));
        }
        self.check(&normalized, ctx.filtered())?;
        Ok(VerifiedPath {
            given: normalized.given,
            final_path: normalized.final_path,
            identity: meta.identity,
            volume: meta.volume,
            intent,
            is_dir: meta.is_dir,
        })
    }

    pub fn reverify(&self, path: &VerifiedPath) -> Result<(), GuardError> {
        let normalized = normalize(
            self.fs.as_ref(),
            self.env.as_ref(),
            self.known.as_ref(),
            &self.cwd,
            path.given(),
        )?;
        let meta = match self.fs.metadata(&normalized.final_path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(GuardError::new(
                    GuardReason::ChangedSinceScan,
                    format!("path disappeared: {}", path.given().display()),
                ));
            }
            Err(error) => return Err(io_error(error)),
        };
        if meta.identity != path.identity || meta.volume != path.volume {
            return Err(GuardError::new(
                GuardReason::ChangedSinceScan,
                format!("path identity changed: {}", path.given().display()),
            ));
        }
        self.check(&normalized, false)?;
        Ok(())
    }

    /// Re-check a scanned item just before it is removed and return a fresh
    /// `VerifiedPath`. A replaced path, a link, or a newly protected path is
    /// refused.
    pub fn reverify_scan(
        &self,
        path: &Path,
        intent: Intent,
        ctx: &RuleCtx,
        identity: FileIdentity,
    ) -> Result<VerifiedPath, GuardError> {
        let normalized = normalize(
            self.fs.as_ref(),
            self.env.as_ref(),
            self.known.as_ref(),
            &self.cwd,
            path,
        )?;
        let meta = match self.fs.metadata(&normalized.final_path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(GuardError::new(
                    GuardReason::ChangedSinceScan,
                    format!("path disappeared: {}", path.display()),
                ));
            }
            Err(error) => return Err(io_error(error)),
        };
        if meta.identity != identity || meta.volume != VolumeKind::Fixed {
            return Err(GuardError::new(
                GuardReason::ChangedSinceScan,
                format!("path identity changed: {}", path.display()),
            ));
        }
        self.check(&normalized, ctx.filtered())?;
        Ok(VerifiedPath {
            given: normalized.given,
            final_path: normalized.final_path,
            identity: meta.identity,
            volume: meta.volume,
            intent,
            is_dir: meta.is_dir,
        })
    }

    fn check(&self, normalized: &NormalizedPath, filtered: bool) -> Result<(), GuardError> {
        self.table
            .check(&normalized.given.to_string_lossy(), filtered)?;
        self.table
            .check(&normalized.final_path.to_string_lossy(), filtered)?;
        if !browser_profile_allows(&normalized.given) {
            return Err(GuardError::new(
                GuardReason::Protected,
                format!(
                    "browser profile target is not allowed: {}",
                    normalized.given.display()
                ),
            ));
        }
        Ok(())
    }
}

fn io_error(error: io::Error) -> GuardError {
    let reason = match error.kind() {
        io::ErrorKind::NotFound => GuardReason::NotFound,
        io::ErrorKind::PermissionDenied => GuardReason::AccessDenied,
        _ => GuardReason::Unsupported,
    };
    GuardError::new(reason, error.to_string())
}

pub fn identity_of(meta: &rscleaner_platform::FsMeta) -> FileIdentity {
    meta.identity
}

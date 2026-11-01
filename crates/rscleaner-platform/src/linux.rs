#![cfg(target_os = "linux")]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use directories::UserDirs;
use sysinfo::{ProcessesToUpdate, System};

use crate::elevation::Elevation;
use crate::env::{Env, SystemEnv};
use crate::fs::{DirEntry, FileIdentity, Fs, FsMeta, TrashError, VolumeKind};
use crate::known::{KnownFolder, KnownFolders};
use crate::procs::Procs;
use crate::system::{OsInfo, SystemInfo};

#[derive(Clone, Debug, Default)]
pub struct LinuxFs;

impl LinuxFs {
    pub fn new() -> Self {
        Self
    }
}

impl Fs for LinuxFs {
    fn metadata(&self, path: &Path) -> io::Result<FsMeta> {
        let meta = std::fs::symlink_metadata(path)?;
        Ok(FsMeta {
            len: meta.len(),
            modified: meta.modified().ok(),
            is_dir: meta.is_dir(),
            is_file: meta.is_file(),
            is_symlink: meta.file_type().is_symlink(),
            readonly: meta.permissions().readonly(),
            identity: FileIdentity {
                volume: meta.dev(),
                file: meta.ino(),
            },
            volume: volume_kind(path),
        })
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            let meta = self.metadata(&path)?;
            entries.push(DirEntry { path, meta });
        }
        Ok(entries)
    }

    fn long_path(&self, path: &Path) -> io::Result<PathBuf> {
        Ok(path.to_path_buf())
    }

    fn remove_file_raw(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn remove_dir_raw(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir(path)
    }

    fn trash_raw(&self, path: &Path) -> Result<(), TrashError> {
        trash::delete(path).map_err(|error| TrashError(error.to_string()))
    }

    fn remove_on_reboot_raw(&self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "deferred deletion is not supported on Linux",
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub struct LinuxProcs;

impl Procs for LinuxProcs {
    fn running(&self, names: &[String]) -> io::Result<Vec<String>> {
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::All, true);
        let running: Vec<String> = system
            .processes()
            .values()
            .map(|process| process.name().to_string_lossy().to_string())
            .collect();
        Ok(names
            .iter()
            .filter(|name| running.iter().any(|running| running == *name))
            .cloned()
            .collect())
    }
}

#[derive(Clone, Debug, Default)]
pub struct LinuxElevation;

impl Elevation for LinuxElevation {
    fn is_elevated(&self) -> bool {
        rustix::process::geteuid().is_root()
    }

    fn relaunch_elevated(&self, _args: &[OsString]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "use sudo from the TUI",
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub struct LinuxSystemInfo;

impl SystemInfo for LinuxSystemInfo {
    fn os(&self) -> OsInfo {
        read_os_release()
    }

    fn volume(&self, path: &Path) -> io::Result<VolumeKind> {
        Ok(volume_kind(path))
    }
}

#[derive(Clone, Debug, Default)]
pub struct LinuxKnownFolders;

impl KnownFolders for LinuxKnownFolders {
    fn get(&self, id: KnownFolder) -> Option<PathBuf> {
        let dirs = UserDirs::new()?;
        match id {
            KnownFolder::Downloads => dirs.download_dir().map(Path::to_path_buf),
            KnownFolder::Documents => dirs.document_dir().map(Path::to_path_buf),
            KnownFolder::Desktop => dirs.desktop_dir().map(Path::to_path_buf),
            KnownFolder::Pictures => dirs.picture_dir().map(Path::to_path_buf),
            KnownFolder::Videos => dirs.video_dir().map(Path::to_path_buf),
            KnownFolder::Music => dirs.audio_dir().map(Path::to_path_buf),
        }
    }
}

pub type LinuxEnv = SystemEnv;

fn volume_kind(path: &Path) -> VolumeKind {
    let pseudo = ["/proc", "/sys", "/dev", "/run"];
    if pseudo
        .iter()
        .any(|prefix| path == Path::new(prefix) || path.starts_with(prefix))
    {
        VolumeKind::Other
    } else {
        VolumeKind::Fixed
    }
}

fn read_os_release() -> OsInfo {
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            );
        }
    }
    let name = fields
        .get("PRETTY_NAME")
        .or_else(|| fields.get("NAME"))
        .cloned()
        .unwrap_or_else(|| "Linux".into());
    let mut id = Vec::new();
    if let Some(value) = fields.get("ID") {
        id.push(value.clone());
    }
    if let Some(value) = fields.get("ID_LIKE") {
        id.extend(value.split_whitespace().map(str::to_string));
    }
    id.sort();
    id.dedup();
    OsInfo {
        name,
        id,
        version: fields.get("VERSION_ID").cloned(),
        build: None,
    }
}

pub fn env() -> &'static dyn Env {
    static ENV: SystemEnv = SystemEnv;
    &ENV
}

#![cfg(windows)]

use std::ffi::{OsString, c_void};
use std::io;
use std::mem::size_of;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};

use directories::UserDirs;
use sysinfo::{ProcessesToUpdate, System};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{
    GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    GetDriveTypeW, GetFileInformationByHandle, GetLongPathNameW, MOVEFILE_DELAY_UNTIL_REBOOT,
    MoveFileExW,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use crate::elevation::Elevation;
use crate::env::{Env, SystemEnv};
use crate::fs::{DirEntry, FileIdentity, Fs, FsMeta, TrashError, VolumeKind};
use crate::known::{KnownFolder, KnownFolders};
use crate::procs::Procs;
use crate::system::{OsInfo, SystemInfo};

#[derive(Clone, Debug, Default)]
pub struct WindowsFs;

impl WindowsFs {
    pub fn new() -> Self {
        Self
    }
}

impl Fs for WindowsFs {
    fn metadata(&self, path: &Path) -> io::Result<FsMeta> {
        let meta = std::fs::symlink_metadata(path)?;
        let identity = identity_for(path)?;
        Ok(FsMeta {
            len: meta.len(),
            modified: meta.modified().ok(),
            is_dir: meta.is_dir(),
            is_file: meta.is_file(),
            is_symlink: meta.file_type().is_symlink(),
            readonly: meta.permissions().readonly(),
            identity,
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
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut buffer = vec![0u16; 32_768];
        // SAFETY: `wide` is NUL-terminated and `buffer` has the stated length.
        let length =
            unsafe { GetLongPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
        if length == 0 {
            return Ok(path.to_path_buf());
        }
        Ok(PathBuf::from(OsString::from_wide(
            &buffer[..length as usize],
        )))
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

    fn remove_on_reboot_raw(&self, path: &Path) -> io::Result<()> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: `wide` is NUL-terminated; the flags only schedule deletion.
        let result =
            unsafe { MoveFileExW(wide.as_ptr(), std::ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT) };
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindowsProcs;

impl Procs for WindowsProcs {
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
            .filter(|name| {
                running
                    .iter()
                    .any(|running| running.eq_ignore_ascii_case(name))
            })
            .cloned()
            .collect())
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindowsElevation;

impl Elevation for WindowsElevation {
    fn is_elevated(&self) -> bool {
        let mut token: HANDLE = std::ptr::null_mut();
        // SAFETY: the current process handle is valid for the call.
        let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
        if opened == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0u32;
        // SAFETY: `elevation` points to a correctly sized TOKEN_ELEVATION.
        let queried = unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut c_void,
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            )
        };
        // SAFETY: the token handle came from OpenProcessToken.
        unsafe { CloseHandle(token) };
        queried != 0 && elevation.TokenIsElevated != 0
    }

    fn relaunch_elevated(&self, _args: &[OsString]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "elevated relaunch is implemented in M4",
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindowsSystemInfo;

impl SystemInfo for WindowsSystemInfo {
    fn os(&self) -> OsInfo {
        OsInfo {
            name: "Windows".into(),
            id: vec!["windows".into()],
            version: None,
            build: None,
        }
    }

    fn volume(&self, path: &Path) -> io::Result<VolumeKind> {
        Ok(volume_kind(path))
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindowsKnownFolders;

impl KnownFolders for WindowsKnownFolders {
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

pub type WindowsEnv = SystemEnv;

fn identity_for(path: &Path) -> io::Result<FileIdentity> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the file handle is valid while `file` is alive; `info` is initialized.
    let result = unsafe { GetFileInformationByHandle(file.as_raw_handle() as HANDLE, &mut info) };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(FileIdentity {
        volume: info.dwVolumeSerialNumber as u64,
        file: ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
    })
}

fn volume_kind(path: &Path) -> VolumeKind {
    let Some(component) = path.components().next() else {
        return VolumeKind::Other;
    };
    let root = component.as_os_str();
    let wide: Vec<u16> = root.encode_wide().chain(std::iter::once(0)).collect();
    // SAFETY: `wide` is NUL-terminated.
    let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
    match kind {
        2 => VolumeKind::Removable,
        3 => VolumeKind::Fixed,
        4 => VolumeKind::Network,
        _ => VolumeKind::Other,
    }
}

pub fn env() -> &'static dyn Env {
    &SystemEnv
}

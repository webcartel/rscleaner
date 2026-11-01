#![allow(unsafe_code)]

//! Platform traits and in-memory fakes. OS bindings are confined to this crate.

pub mod elevation;
pub mod env;
pub mod fake;
pub mod fs;
pub mod known;
pub mod procs;
pub mod system;

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(windows)]
pub use windows::{
    WindowsElevation, WindowsEnv, WindowsFs, WindowsKnownFolders, WindowsProcs, WindowsSystemInfo,
};

#[cfg(target_os = "linux")]
pub use linux::{
    LinuxElevation, LinuxEnv, LinuxFs, LinuxKnownFolders, LinuxProcs, LinuxSystemInfo,
};

pub use elevation::{Elevation, FakeElevation};
pub use env::{Env, FakeEnv, SystemEnv};
pub use fake::FakeFs;
pub use fs::{DirEntry, FileIdentity, Fs, FsMeta, TrashError, VolumeKind};
pub use known::{EmptyKnownFolders, FakeKnownFolders, KnownFolder, KnownFolders};
pub use procs::{FakeProcs, Procs};
pub use system::{FakeSystemInfo, OsInfo, SystemInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformKind {
    Windows,
    Linux,
}

impl PlatformKind {
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

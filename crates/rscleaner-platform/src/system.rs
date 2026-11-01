use std::io;
use std::path::Path;

use crate::fs::VolumeKind;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OsInfo {
    pub name: String,
    pub id: Vec<String>,
    pub version: Option<String>,
    pub build: Option<u32>,
}

pub trait SystemInfo: Send + Sync {
    fn os(&self) -> OsInfo;
    fn volume(&self, path: &Path) -> io::Result<VolumeKind>;
}

#[derive(Clone, Debug, Default)]
pub struct FakeSystemInfo {
    pub os: OsInfo,
    pub volume: VolumeKind,
}

impl FakeSystemInfo {
    pub fn new(os: OsInfo, volume: VolumeKind) -> Self {
        Self { os, volume }
    }
}

impl SystemInfo for FakeSystemInfo {
    fn os(&self) -> OsInfo {
        self.os.clone()
    }

    fn volume(&self, _path: &Path) -> io::Result<VolumeKind> {
        Ok(self.volume)
    }
}

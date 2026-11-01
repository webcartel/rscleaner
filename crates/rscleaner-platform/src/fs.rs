use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    pub volume: u64,
    pub file: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VolumeKind {
    #[default]
    Fixed,
    Removable,
    Network,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FsMeta {
    pub len: u64,
    pub modified: Option<SystemTime>,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub readonly: bool,
    pub identity: FileIdentity,
    pub volume: VolumeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub path: PathBuf,
    pub meta: FsMeta,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashError(pub String);

impl fmt::Display for TrashError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for TrashError {}

pub trait Fs: Send + Sync {
    fn metadata(&self, path: &Path) -> io::Result<FsMeta>;
    fn read_link(&self, path: &Path) -> io::Result<PathBuf>;
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>>;
    fn long_path(&self, path: &Path) -> io::Result<PathBuf>;
    fn remove_file_raw(&self, path: &Path) -> io::Result<()>;
    fn remove_dir_raw(&self, path: &Path) -> io::Result<()>;
    fn remove_tree_raw(&self, path: &Path) -> io::Result<()>;
    fn trash_raw(&self, path: &Path) -> Result<(), TrashError>;
    fn remove_on_reboot_raw(&self, path: &Path) -> io::Result<()>;
}

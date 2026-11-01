use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::fs::{DirEntry, FileIdentity, Fs, FsMeta, TrashError, VolumeKind};

#[derive(Clone, Debug)]
struct FakeNode {
    meta: FsMeta,
    link_target: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct FakeState {
    volume: VolumeKind,
    next_file: u64,
    nodes: BTreeMap<PathBuf, FakeNode>,
}

#[derive(Clone, Debug)]
pub struct FakeFs {
    state: Arc<Mutex<FakeState>>,
}

impl FakeFs {
    pub fn new(volume: VolumeKind) -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeState {
                volume,
                next_file: 1,
                nodes: BTreeMap::new(),
            })),
        }
    }

    fn key(path: &Path) -> PathBuf {
        let text = path.to_string_lossy();
        text.strip_prefix("\\\\?\\")
            .map(PathBuf::from)
            .unwrap_or_else(|| path.to_path_buf())
    }

    pub fn add_file(&self, path: &Path, bytes: &[u8], identity_volume: u64) {
        let mut state = self.state();
        let volume = state.volume;
        let file = state.next_file;
        state.next_file += 1;
        state.nodes.insert(
            Self::key(path),
            FakeNode {
                meta: FsMeta {
                    len: bytes.len() as u64,
                    modified: None,
                    is_dir: false,
                    is_file: true,
                    is_symlink: false,
                    readonly: false,
                    identity: FileIdentity {
                        volume: identity_volume,
                        file,
                    },
                    volume,
                },
                link_target: None,
            },
        );
    }

    pub fn add_hard_link(&self, path: &Path, existing: &Path) {
        let mut state = self.state();
        let Some(node) = state.nodes.get(&Self::key(existing)).cloned() else {
            return;
        };
        state.nodes.insert(Self::key(path), node);
    }

    pub fn set_modified(&self, path: &Path, unix_seconds: i64) {
        let mut state = self.state();
        let key = Self::key(path);
        if let Some(node) = state.nodes.get_mut(&key) {
            node.meta.modified = std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(
                u64::try_from(unix_seconds).unwrap_or(0),
            ));
        }
    }

    pub fn add_dir(&self, path: &Path) {
        let mut state = self.state();
        let volume = state.volume;
        let file = state.next_file;
        state.next_file += 1;
        state.nodes.insert(
            Self::key(path),
            FakeNode {
                meta: FsMeta {
                    len: 0,
                    modified: None,
                    is_dir: true,
                    is_file: false,
                    is_symlink: false,
                    readonly: false,
                    identity: FileIdentity { volume: 0, file },
                    volume,
                },
                link_target: None,
            },
        );
    }

    pub fn add_symlink(&self, path: &Path, target: &Path) {
        let mut state = self.state();
        let volume = state.volume;
        let file = state.next_file;
        state.next_file += 1;
        state.nodes.insert(
            Self::key(path),
            FakeNode {
                meta: FsMeta {
                    len: 0,
                    modified: None,
                    is_dir: false,
                    is_file: false,
                    is_symlink: true,
                    readonly: false,
                    identity: FileIdentity { volume: 0, file },
                    volume,
                },
                link_target: Some(target.to_path_buf()),
            },
        );
    }

    fn state(&self) -> MutexGuard<'_, FakeState> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl Fs for FakeFs {
    fn metadata(&self, path: &Path) -> io::Result<FsMeta> {
        let state = self.state();
        state
            .nodes
            .get(&Self::key(path))
            .map(|node| node.meta.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "fake path not found"))
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        let state = self.state();
        state
            .nodes
            .get(&Self::key(path))
            .and_then(|node| node.link_target.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "fake link not found"))
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let state = self.state();
        let parent = Self::key(path);
        Ok(state
            .nodes
            .iter()
            .filter(|(candidate, _)| candidate.parent() == Some(parent.as_path()))
            .map(|(candidate, node)| DirEntry {
                path: candidate.clone(),
                meta: node.meta.clone(),
            })
            .collect())
    }

    fn long_path(&self, path: &Path) -> io::Result<PathBuf> {
        Ok(path.to_path_buf())
    }

    fn remove_file_raw(&self, path: &Path) -> io::Result<()> {
        let mut state = self.state();
        let key = Self::key(path);
        match state.nodes.get(&key) {
            Some(node) if !node.meta.is_dir => {
                state.nodes.remove(&key);
                Ok(())
            }
            Some(_) => Err(io::Error::new(io::ErrorKind::IsADirectory, "not a file")),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "fake path not found",
            )),
        }
    }

    fn remove_dir_raw(&self, path: &Path) -> io::Result<()> {
        let mut state = self.state();
        let key = Self::key(path);
        match state.nodes.get(&key) {
            Some(node) if node.meta.is_dir => {
                state.nodes.remove(&key);
                Ok(())
            }
            Some(_) => Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "not a directory",
            )),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "fake path not found",
            )),
        }
    }

    fn remove_tree_raw(&self, path: &Path) -> io::Result<()> {
        let mut state = self.state();
        let key = Self::key(path);
        if !state.nodes.contains_key(&key) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "fake path not found",
            ));
        }
        state
            .nodes
            .retain(|candidate, _| candidate != &key && !candidate.starts_with(&key));
        Ok(())
    }

    fn trash_raw(&self, path: &Path) -> Result<(), TrashError> {
        self.remove_file_raw(path)
            .or_else(|_| self.remove_dir_raw(path))
            .map_err(|error| TrashError(error.to_string()))
    }

    fn remove_on_reboot_raw(&self, path: &Path) -> io::Result<()> {
        self.remove_file_raw(path)
            .or_else(|_| self.remove_dir_raw(path))
    }
}

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::fs::{DirEntry, FileIdentity, Fs, FsMeta, TrashError, VolumeKind};

#[derive(Clone, Debug)]
struct FakeNode {
    meta: FsMeta,
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

    pub fn add_file(&self, path: &Path, bytes: &[u8], identity_volume: u64) {
        let mut state = self.state();
        let volume = state.volume;
        let file = state.next_file;
        state.next_file += 1;
        state.nodes.insert(
            path.to_path_buf(),
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
            },
        );
    }

    pub fn add_dir(&self, path: &Path) {
        let mut state = self.state();
        let volume = state.volume;
        let file = state.next_file;
        state.next_file += 1;
        state.nodes.insert(
            path.to_path_buf(),
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
            },
        );
    }

    pub fn add_symlink(&self, path: &Path) {
        let mut state = self.state();
        let volume = state.volume;
        let file = state.next_file;
        state.next_file += 1;
        state.nodes.insert(
            path.to_path_buf(),
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
            .get(path)
            .map(|node| node.meta.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "fake path not found"))
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let state = self.state();
        Ok(state
            .nodes
            .iter()
            .filter(|(candidate, _)| candidate.parent() == Some(path))
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
        match state.nodes.get(path) {
            Some(node) if !node.meta.is_dir => {
                state.nodes.remove(path);
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
        match state.nodes.get(path) {
            Some(node) if node.meta.is_dir => {
                state.nodes.remove(path);
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

#![forbid(unsafe_code)]

//! Platform traits and in-memory fakes.

pub mod env;
pub mod known;

pub use env::{Env, FakeEnv, SystemEnv};
pub use known::{EmptyKnownFolders, FakeKnownFolders, KnownFolder, KnownFolders};

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

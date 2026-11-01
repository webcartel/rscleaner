use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KnownFolder {
    Downloads,
    Documents,
    Desktop,
    Pictures,
    Videos,
    Music,
}

pub trait KnownFolders: Send + Sync {
    fn get(&self, id: KnownFolder) -> Option<PathBuf>;
}

#[derive(Clone, Debug, Default)]
pub struct FakeKnownFolders {
    values: BTreeMap<KnownFolder, PathBuf>,
}

impl FakeKnownFolders {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<const N: usize> From<[(KnownFolder, &str); N]> for FakeKnownFolders {
    fn from(values: [(KnownFolder, &str); N]) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(folder, path)| (folder, PathBuf::from(path)))
                .collect(),
        }
    }
}

impl KnownFolders for FakeKnownFolders {
    fn get(&self, id: KnownFolder) -> Option<PathBuf> {
        self.values.get(&id).cloned()
    }
}

#[derive(Clone, Debug, Default)]
pub struct EmptyKnownFolders;

impl KnownFolders for EmptyKnownFolders {
    fn get(&self, _id: KnownFolder) -> Option<PathBuf> {
        None
    }
}

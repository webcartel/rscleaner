use std::collections::BTreeSet;
use std::io;

pub trait Procs: Send + Sync {
    fn running(&self, names: &[String]) -> io::Result<Vec<String>>;
}

#[derive(Clone, Debug, Default)]
pub struct FakeProcs {
    running: BTreeSet<String>,
}

impl FakeProcs {
    pub fn new(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            running: names.into_iter().collect(),
        }
    }
}

impl Procs for FakeProcs {
    fn running(&self, names: &[String]) -> io::Result<Vec<String>> {
        Ok(names
            .iter()
            .filter(|name| self.running.contains(*name))
            .cloned()
            .collect())
    }
}

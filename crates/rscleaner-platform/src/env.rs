use std::collections::BTreeMap;
use std::ffi::OsString;

pub trait Env: Send + Sync {
    fn get(&self, name: &str) -> Option<OsString>;
}

#[derive(Clone, Debug, Default)]
pub struct FakeEnv {
    values: BTreeMap<String, OsString>,
}

impl FakeEnv {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<const N: usize> From<[(&str, &str); N]> for FakeEnv {
    fn from(values: [(&str, &str); N]) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(name, value)| (name.to_string(), OsString::from(value)))
                .collect(),
        }
    }
}

impl Env for FakeEnv {
    fn get(&self, name: &str) -> Option<OsString> {
        self.values.get(name).cloned()
    }
}

#[derive(Clone, Debug, Default)]
pub struct SystemEnv;

impl Env for SystemEnv {
    fn get(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }
}

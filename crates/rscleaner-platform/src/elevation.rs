use std::ffi::OsString;
use std::io;

pub trait Elevation: Send + Sync {
    fn is_elevated(&self) -> bool;
    fn relaunch_elevated(&self, args: &[OsString]) -> io::Result<()>;
}

#[derive(Clone, Debug, Default)]
pub struct FakeElevation {
    elevated: bool,
}

impl FakeElevation {
    pub fn new(elevated: bool) -> Self {
        Self { elevated }
    }
}

impl Elevation for FakeElevation {
    fn is_elevated(&self) -> bool {
        self.elevated
    }

    fn relaunch_elevated(&self, _args: &[OsString]) -> io::Result<()> {
        Ok(())
    }
}

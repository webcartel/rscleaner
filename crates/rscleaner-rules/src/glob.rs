use globset::{Glob, GlobMatcher};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobError(pub String);

impl std::fmt::Display for GlobError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for GlobError {}

#[derive(Clone, Debug)]
pub struct CompiledGlob {
    matcher: GlobMatcher,
}

impl CompiledGlob {
    pub fn compile(pattern: &str) -> Result<Self, GlobError> {
        if pattern.trim().is_empty() {
            return Err(GlobError("empty glob".into()));
        }
        let normalized = pattern.replace('\\', "/");
        let glob = Glob::new(&normalized).map_err(|error| GlobError(error.to_string()))?;
        Ok(Self {
            matcher: glob.compile_matcher(),
        })
    }

    pub fn is_match(&self, path: &str) -> bool {
        self.matcher.is_match(path.replace('\\', "/"))
    }
}

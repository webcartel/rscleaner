use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Yaml,
    Schema,
    UnknownField,
    UnknownVar,
    EmptyTargets,
    EmptyMask,
    BadGlob,
    DuplicateId,
    IdPattern,
    PlatformWhen,
    RegistryUser,
    RiskUnderstated,
    CommandDisabled,
    ProtectedTarget,
    UnknownPlatform,
    TargetKind,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yaml => "E_YAML",
            Self::Schema => "E_SCHEMA",
            Self::UnknownField => "E_UNKNOWN_FIELD",
            Self::UnknownVar => "E_UNKNOWN_VAR",
            Self::EmptyTargets => "E_EMPTY_TARGETS",
            Self::EmptyMask => "E_EMPTY_MASK",
            Self::BadGlob => "E_BAD_GLOB",
            Self::DuplicateId => "E_DUPLICATE_ID",
            Self::IdPattern => "E_ID_PATTERN",
            Self::PlatformWhen => "E_PLATFORM_WHEN",
            Self::RegistryUser => "E_REGISTRY_USER",
            Self::RiskUnderstated => "E_RISK_UNDERSTATED",
            Self::CommandDisabled => "E_COMMAND_DISABLED",
            Self::ProtectedTarget => "E_PROTECTED_TARGET",
            Self::UnknownPlatform => "E_UNKNOWN_PLATFORM",
            Self::TargetKind => "E_TARGET_KIND",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    RuleReplaced,
    UnknownToggle,
    UnknownProfile,
    ProtectedTarget,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RuleError {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub code: ErrorCode,
    pub message: String,
}

impl RuleError {
    pub fn new(
        file: impl Into<String>,
        line: usize,
        column: usize,
        code: ErrorCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for RuleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}: {}: {}",
            self.file,
            self.line,
            self.column,
            self.code.as_str(),
            self.message
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RuleWarning {
    pub code: WarningCode,
    pub message: String,
}

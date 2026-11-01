use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardReason {
    Protected,
    NotFixedVolume,
    NotFound,
    AccessDenied,
    InvalidPath,
    Unsupported,
    ChangedSinceScan,
    CloudPlaceholder,
}

impl GuardReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Protected => "protected",
            Self::NotFixedVolume => "not_fixed_volume",
            Self::NotFound => "not_found",
            Self::AccessDenied => "access_denied",
            Self::InvalidPath => "invalid_path",
            Self::Unsupported => "unsupported",
            Self::ChangedSinceScan => "changed_since_scan",
            Self::CloudPlaceholder => "cloud_placeholder",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardError {
    pub reason: GuardReason,
    pub message: String,
}

impl GuardError {
    pub fn new(reason: GuardReason, message: impl Into<String>) -> Self {
        Self {
            reason,
            message: message.into(),
        }
    }

    /// The plan and report reason for a refused path.
    pub fn skip_reason(&self) -> rscleaner_model::SkipReason {
        match self.reason {
            GuardReason::Protected => rscleaner_model::SkipReason::Protected,
            GuardReason::NotFixedVolume => rscleaner_model::SkipReason::UnsupportedHere,
            GuardReason::NotFound => rscleaner_model::SkipReason::NotFound,
            GuardReason::AccessDenied => rscleaner_model::SkipReason::AccessDenied,
            GuardReason::InvalidPath => rscleaner_model::SkipReason::NotApplicable,
            GuardReason::Unsupported => rscleaner_model::SkipReason::UnsupportedHere,
            GuardReason::ChangedSinceScan => rscleaner_model::SkipReason::ChangedSinceScan,
            GuardReason::CloudPlaceholder => rscleaner_model::SkipReason::CloudPlaceholder,
        }
    }
}

impl fmt::Display for GuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.reason.as_str(), self.message)
    }
}

impl std::error::Error for GuardError {}

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    #[default]
    Safe,
    Moderate,
    Dangerous,
}

impl Risk {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Moderate => "moderate",
            Self::Dangerous => "dangerous",
        }
    }

    pub fn max(self, other: Self) -> Self {
        if self >= other { self } else { other }
    }
}

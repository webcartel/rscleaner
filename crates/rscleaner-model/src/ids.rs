use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct RuleId(String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleIdError {
    Empty,
    Invalid,
}

impl fmt::Display for RuleIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("rule id is empty"),
            Self::Invalid => formatter.write_str("rule id must match ^[a-z0-9]+(\\.[a-z0-9-]+)+$"),
        }
    }
}

impl std::error::Error for RuleIdError {}

impl FromStr for RuleId {
    type Err = RuleIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(RuleIdError::Empty);
        }
        let parts: Vec<&str> = value.split('.').collect();
        if parts.len() < 2 {
            return Err(RuleIdError::Invalid);
        }
        for (index, part) in parts.iter().enumerate() {
            if part.is_empty() {
                return Err(RuleIdError::Invalid);
            }
            let valid = if index == 0 {
                part.chars()
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
            } else {
                part.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
                })
            };
            if !valid {
                return Err(RuleIdError::Invalid);
            }
        }
        Ok(Self(value.to_string()))
    }
}

impl RuleId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for RuleId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for RuleId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

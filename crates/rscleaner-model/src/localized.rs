use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Localized {
    Text(String),
    Map(BTreeMap<String, String>),
}

impl Localized {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn resolve(&self, language: &str) -> &str {
        match self {
            Self::Text(value) => value,
            Self::Map(values) => values
                .get(language)
                .or_else(|| values.get("en"))
                .or_else(|| values.values().next())
                .map(String::as_str)
                .unwrap_or(""),
        }
    }
}

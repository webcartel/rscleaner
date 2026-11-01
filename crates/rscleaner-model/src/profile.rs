use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::risk::Risk;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub include_tags: Vec<String>,
    #[serde(default)]
    pub include_categories: Vec<String>,
    pub max_risk: Option<Risk>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

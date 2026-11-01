use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ids::RuleId;
use crate::platform::Theme;
use crate::profile::Profile;
use crate::rule::ByteSize;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpaceConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
    pub min_size: Option<ByteSize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashConfig {
    #[serde(default)]
    pub default: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleToggle {
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub rules: BTreeMap<RuleId, RuleToggle>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    #[serde(default)]
    pub allow_user_commands: bool,
    #[serde(default)]
    pub search_roots: Vec<String>,
    #[serde(default)]
    pub space: SpaceConfig,
    #[serde(default)]
    pub trash: TrashConfig,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: None,
            theme: Theme::Dark,
            rules: BTreeMap::new(),
            exclude_paths: Vec::new(),
            allow_user_commands: false,
            search_roots: Vec::new(),
            space: SpaceConfig::default(),
            trash: TrashConfig::default(),
            profiles: BTreeMap::new(),
        }
    }
}

use std::fmt;

use schemars::JsonSchema;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use crate::ids::RuleId;
use crate::localized::Localized;
use crate::platform::{Mode, Platform};
use crate::risk::Risk;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PathRepr {
    pub display: String,
    pub raw_b64: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum RuleSource {
    #[default]
    Builtin,
    UserFile(PathRepr),
    ExtraDir(PathRepr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ByteSize(pub u64);

impl<'de> Deserialize<'de> for ByteSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Int(u64),
            Text(String),
        }

        match Repr::deserialize(deserializer)? {
            Repr::Int(value) => Ok(Self(value)),
            Repr::Text(value) => parse_byte_size(&value).map(Self).map_err(de::Error::custom),
        }
    }
}

impl fmt::Display for ByteSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

fn parse_byte_size(value: &str) -> Result<u64, String> {
    let text = value.trim().to_ascii_uppercase();
    if text.is_empty() {
        return Err("empty byte size".into());
    }
    let (number, multiplier) = if let Some(number) = text.strip_suffix("TB") {
        (number, 1024u64.pow(4))
    } else if let Some(number) = text.strip_suffix("GB") {
        (number, 1024u64.pow(3))
    } else if let Some(number) = text.strip_suffix("MB") {
        (number, 1024u64.pow(2))
    } else if let Some(number) = text.strip_suffix("KB") {
        (number, 1024)
    } else if let Some(number) = text.strip_suffix('B') {
        (number, 1)
    } else {
        (text.as_str(), 1)
    };
    let number = number.trim();
    let value: u64 = number
        .parse()
        .map_err(|_| format!("invalid byte size: {value}"))?;
    value
        .checked_mul(multiplier)
        .ok_or_else(|| format!("byte size overflows: {value}"))
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct When {
    #[serde(default)]
    pub distro: Vec<String>,
    #[serde(default)]
    pub command_exists: Vec<String>,
    #[serde(default)]
    pub path_exists: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegistryTarget {
    pub check: String,
    pub hive: String,
    pub key: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    Path {
        path: String,
        #[serde(default)]
        mode: Mode,
        #[serde(default)]
        include: Vec<String>,
        #[serde(default)]
        exclude: Vec<String>,
        min_age_days: Option<u32>,
        min_size: Option<ByteSize>,
    },
    Find {
        #[serde(default)]
        roots: Vec<String>,
        name: String,
        #[serde(default)]
        has_sibling: Vec<String>,
        max_depth: Option<u8>,
        #[serde(default)]
        exclude: Vec<String>,
        min_age_days: Option<u32>,
    },
    Command {
        cmd: String,
        #[serde(default)]
        args: Vec<String>,
        timeout_s: Option<u32>,
    },
    Registry(RegistryTarget),
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TargetRepr {
    Path {
        path: String,
        #[serde(default)]
        mode: Mode,
        #[serde(default)]
        include: Vec<String>,
        #[serde(default)]
        exclude: Vec<String>,
        min_age_days: Option<u32>,
        min_size: Option<ByteSize>,
    },
    Find {
        #[serde(default)]
        roots: Vec<String>,
        name: String,
        #[serde(default)]
        has_sibling: Vec<String>,
        max_depth: Option<u8>,
        #[serde(default)]
        exclude: Vec<String>,
        min_age_days: Option<u32>,
    },
    Command {
        cmd: String,
        #[serde(default)]
        args: Vec<String>,
        timeout_s: Option<u32>,
    },
    Registry(RegistryTarget),
}

impl From<TargetRepr> for Target {
    fn from(value: TargetRepr) -> Self {
        match value {
            TargetRepr::Path {
                path,
                mode,
                include,
                exclude,
                min_age_days,
                min_size,
            } => Self::Path {
                path,
                mode,
                include,
                exclude,
                min_age_days,
                min_size,
            },
            TargetRepr::Find {
                roots,
                name,
                has_sibling,
                max_depth,
                exclude,
                min_age_days,
            } => Self::Find {
                roots,
                name,
                has_sibling,
                max_depth,
                exclude,
                min_age_days,
            },
            TargetRepr::Command {
                cmd,
                args,
                timeout_s,
            } => Self::Command {
                cmd,
                args,
                timeout_s,
            },
            TargetRepr::Registry(target) => Self::Registry(target),
        }
    }
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut value = serde_json::Value::deserialize(deserializer)?;
        if let serde_json::Value::Object(map) = &mut value {
            map.entry("kind")
                .or_insert_with(|| serde_json::Value::String("path".into()));
        }
        serde_json::from_value::<TargetRepr>(value)
            .map(Target::from)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: RuleId,
    pub name: Localized,
    pub description: Option<Localized>,
    pub category: String,
    pub risk: Risk,
    #[serde(default)]
    pub default_enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    pub platforms: Vec<Platform>,
    pub when: Option<When>,
    #[serde(default)]
    pub requires_admin: bool,
    #[serde(default)]
    pub requires_closed: Vec<String>,
    pub os_min_build: Option<u32>,
    pub trash: Option<bool>,
    pub targets: Vec<Target>,
    #[serde(skip)]
    #[schemars(skip)]
    pub source: RuleSource,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleFile {
    pub schema: u32,
    pub rules: Vec<Rule>,
}

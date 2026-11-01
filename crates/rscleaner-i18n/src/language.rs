use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    Russian,
}

impl Language {
    pub fn from_tag(tag: &str) -> Self {
        let primary = tag
            .split(['-', '_'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if primary == "ru" {
            Self::Russian
        } else {
            Self::English
        }
    }

    pub fn detect(env: &dyn Fn(&str) -> Option<String>) -> Self {
        for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Some(value) = env(name) {
                if !value.is_empty() {
                    return Self::from_tag(&value);
                }
            }
        }
        Self::English
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Russian => "ru",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl FromStr for Language {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "en" => Ok(Self::English),
            "ru" => Ok(Self::Russian),
            _ => Err(()),
        }
    }
}

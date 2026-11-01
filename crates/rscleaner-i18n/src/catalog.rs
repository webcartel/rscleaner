use std::collections::BTreeMap;

use crate::language::Language;

const ENGLISH: &str = include_str!("../locales/en.json");
const RUSSIAN: &str = include_str!("../locales/ru.json");

#[derive(Clone, Debug)]
pub struct Translator {
    language: Language,
    english: BTreeMap<String, String>,
    russian: BTreeMap<String, String>,
}

impl Translator {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            english: parse_catalog(ENGLISH),
            russian: parse_catalog(RUSSIAN),
        }
    }

    pub fn with_overrides(
        language: Language,
        entries: impl IntoIterator<Item = (&'static str, &'static str)>,
    ) -> Self {
        let mut translator = Self::new(language);
        for (key, value) in entries {
            translator
                .english
                .insert(key.to_string(), value.to_string());
        }
        translator
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn t(&self, key: &str) -> String {
        let localized = match self.language {
            Language::English => self.english.get(key),
            Language::Russian => self.russian.get(key).or_else(|| self.english.get(key)),
        };
        localized.cloned().unwrap_or_else(|| key.to_string())
    }

    pub fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut text = self.t(key);
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }
}

fn parse_catalog(source: &str) -> BTreeMap<String, String> {
    serde_json::from_str(source).unwrap_or_default()
}

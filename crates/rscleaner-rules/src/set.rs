use std::collections::BTreeMap;

use rscleaner_model::{Config, Profile, Risk, Rule, RuleId, RuleWarning, WarningCode};

use crate::glob::CompiledGlob;

#[derive(Clone, Debug)]
pub struct RuleSourceSet {
    pub name: String,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, Default)]
pub struct RuleSet {
    pub rules: BTreeMap<RuleId, Rule>,
    pub profiles: BTreeMap<String, Profile>,
    pub warnings: Vec<RuleWarning>,
}

#[derive(Clone, Debug)]
pub struct SelectResult {
    pub rules: Vec<Rule>,
    pub warnings: Vec<RuleWarning>,
}

pub fn merge_sources(sources: Vec<RuleSourceSet>) -> RuleSet {
    let mut set = RuleSet {
        profiles: builtin_profiles(),
        ..RuleSet::default()
    };

    for source in sources {
        for rule in source.rules {
            if set.rules.insert(rule.id.clone(), rule.clone()).is_some() {
                set.warnings.push(RuleWarning {
                    code: WarningCode::RuleReplaced,
                    message: format!("rule {} was replaced by {}", rule.id, source.name),
                });
            }
        }
    }

    set
}

pub fn select(set: &RuleSet, config: &Config, profile_name: &str) -> SelectResult {
    let mut warnings = Vec::new();
    let profile = config
        .profiles
        .get(profile_name)
        .or_else(|| set.profiles.get(profile_name));

    let Some(profile) = profile else {
        warnings.push(RuleWarning {
            code: WarningCode::UnknownProfile,
            message: format!("unknown profile: {profile_name}"),
        });
        return SelectResult {
            rules: Vec::new(),
            warnings,
        };
    };

    let mut rules = Vec::new();
    for rule in set.rules.values() {
        if profile
            .exclude
            .iter()
            .any(|mask| mask_matches(mask, rule.id.as_str()))
        {
            continue;
        }

        let selected = match config.rules.get(&rule.id) {
            Some(toggle) => toggle.enabled,
            None => selected_by_profile(rule, profile_name, profile),
        };
        if !selected {
            continue;
        }

        if let Some(max_risk) = profile.max_risk {
            if rule.risk > max_risk {
                continue;
            }
        }

        rules.push(rule.clone());
    }

    for id in config.rules.keys() {
        if !set.rules.contains_key(id) {
            warnings.push(RuleWarning {
                code: WarningCode::UnknownToggle,
                message: format!("unknown rule toggle: {id}"),
            });
        }
    }

    SelectResult { rules, warnings }
}

fn builtin_profiles() -> BTreeMap<String, Profile> {
    let mut profiles = BTreeMap::new();
    profiles.insert("default".into(), Profile::default());
    profiles.insert(
        "dev".into(),
        Profile {
            include_tags: vec!["dev".into()],
            ..Profile::default()
        },
    );
    profiles.insert(
        "aggressive".into(),
        Profile {
            max_risk: Some(Risk::Moderate),
            ..Profile::default()
        },
    );
    profiles
}

fn selected_by_profile(rule: &Rule, profile_name: &str, profile: &Profile) -> bool {
    match profile_name {
        "default" => rule.default_enabled,
        "dev" => rule.default_enabled || rule.tags.iter().any(|tag| tag == "dev"),
        "aggressive" => true,
        _ => {
            let has_filters = !profile.include.is_empty()
                || !profile.include_tags.is_empty()
                || !profile.include_categories.is_empty();
            if !has_filters {
                return false;
            }
            let include_ok = profile.include.is_empty()
                || profile
                    .include
                    .iter()
                    .any(|mask| mask_matches(mask, rule.id.as_str()));
            let tags_ok = profile.include_tags.is_empty()
                || profile
                    .include_tags
                    .iter()
                    .any(|tag| rule.tags.iter().any(|rule_tag| rule_tag == tag));
            let categories_ok = profile.include_categories.is_empty()
                || profile
                    .include_categories
                    .iter()
                    .any(|category| category == &rule.category);
            include_ok && tags_ok && categories_ok
        }
    }
}

fn mask_matches(mask: &str, value: &str) -> bool {
    CompiledGlob::compile(mask)
        .map(|glob| glob.is_match(value))
        .unwrap_or(false)
}

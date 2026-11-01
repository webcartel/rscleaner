use rscleaner_model::{Config, Localized, Platform, Risk, Rule, RuleSource};
use rscleaner_rules::set::{RuleSourceSet, merge_sources, select};

fn id(value: &str) -> rscleaner_model::RuleId {
    match value.parse() {
        Ok(id) => id,
        Err(_) => panic!("invalid rule id: {value}"),
    }
}

fn source(name: &str, rules: Vec<Rule>) -> RuleSourceSet {
    RuleSourceSet {
        name: name.into(),
        rules,
    }
}

fn rule(value: &str, risk: Risk) -> Rule {
    Rule {
        id: id(value),
        name: Localized::text(value),
        description: None,
        category: "test".into(),
        risk,
        default_enabled: true,
        tags: vec![],
        platforms: vec![Platform::Windows],
        when: None,
        requires_admin: false,
        requires_closed: vec![],
        os_min_build: None,
        trash: None,
        targets: vec![],
        source: RuleSource::Builtin,
    }
}

#[test]
fn later_source_replaces_the_same_id_with_a_warning() {
    let set = merge_sources(vec![
        source("builtin", vec![rule("a.b", Risk::Safe)]),
        source("user", vec![rule("a.b", Risk::Moderate)]),
    ]);
    assert_eq!(set.rules.len(), 1);
    assert_eq!(set.rules["a.b"].risk, Risk::Moderate);
    assert_eq!(set.warnings.len(), 1);
}

#[test]
fn config_toggle_and_profile_limits_apply() {
    let mut safe = rule("a.safe", Risk::Safe);
    safe.default_enabled = true;
    let mut dangerous = rule("a.dangerous", Risk::Dangerous);
    dangerous.default_enabled = true;
    let mut tagged = rule("a.tagged", Risk::Safe);
    tagged.default_enabled = false;
    tagged.tags = vec!["dev".into()];

    let set = merge_sources(vec![source("builtin", vec![safe, dangerous, tagged])]);

    let selected = select(&set, &Config::default(), "default");
    assert_eq!(selected.rules.len(), 2);

    let mut config = Config::default();
    config
        .rules
        .insert(id("a.safe"), rscleaner_model::RuleToggle { enabled: false });
    let selected = select(&set, &config, "default");
    assert_eq!(selected.rules.len(), 1);
    assert_eq!(selected.rules[0].id.as_str(), "a.dangerous");

    let selected = select(&set, &Config::default(), "aggressive");
    assert_eq!(selected.rules.len(), 2);
    assert!(
        selected
            .rules
            .iter()
            .all(|rule| rule.risk < Risk::Dangerous)
    );

    let selected = select(&set, &Config::default(), "dev");
    assert_eq!(selected.rules.len(), 3);
    assert!(
        selected
            .rules
            .iter()
            .any(|rule| rule.id.as_str() == "a.tagged")
    );
}

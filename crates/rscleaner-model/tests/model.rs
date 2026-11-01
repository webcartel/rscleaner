use rscleaner_model::{Localized, Mode, Platform, Risk, Rule, RuleFile, RuleSource, Target};

#[test]
fn rule_file_round_trips_yaml_shape() {
    let file = RuleFile {
        schema: 1,
        rules: vec![Rule {
            id: "browsers.chrome.cache".parse().expect("id"),
            name: Localized::text("Chrome cache"),
            description: None,
            category: "browsers".into(),
            risk: Risk::Safe,
            default_enabled: true,
            tags: vec![],
            platforms: vec![Platform::Windows],
            when: None,
            requires_admin: false,
            requires_closed: vec!["chrome.exe".into()],
            os_min_build: None,
            trash: None,
            targets: vec![Target::Path {
                path: "${LOCALAPPDATA}/Google/Chrome/User Data/*/Cache".into(),
                mode: Mode::Contents,
                include: vec![],
                exclude: vec![],
                min_age_days: None,
                min_size: None,
            }],
            source: RuleSource::Builtin,
        }],
    };

    let json = serde_json::to_string(&file).expect("serialize");
    assert!(json.contains("\"schema\":1"));
    assert!(json.contains("\"Chrome cache\""));
}

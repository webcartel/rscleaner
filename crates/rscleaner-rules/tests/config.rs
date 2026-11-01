use rscleaner_model::Config;

#[test]
fn config_round_trips_and_rejects_unknown_fields() {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("config.yaml");
    let config = Config::default();

    rscleaner_rules::config_file::save(&path, &config).expect("save");
    let loaded = rscleaner_rules::config_file::load(&path).expect("load");
    assert_eq!(loaded, config);

    std::fs::write(&path, "language: en\nsurprise: true\n").expect("write");
    let errors = rscleaner_rules::config_file::load(&path).expect_err("must fail");
    assert_eq!(errors[0].code.as_str(), "E_UNKNOWN_FIELD");
}

#[test]
fn portable_paths_stay_below_the_base_directory() {
    let dir = tempfile::tempdir().expect("temp");
    let paths = rscleaner_rules::paths::resolve(Some(dir.path()), true).expect("paths");
    assert_eq!(paths.config, dir.path().join("config.yaml"));
    assert_eq!(paths.rules, dir.path().join("rules"));
    assert_eq!(paths.logs, dir.path().join("logs"));
    assert_eq!(paths.cache, dir.path().join("cache"));
    assert_eq!(paths.registry_backups, dir.path().join("registry-backups"));
}

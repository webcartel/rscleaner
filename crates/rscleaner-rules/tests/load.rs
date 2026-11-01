use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/rules")
        .join(name)
}

#[test]
fn minimal_rule_file_loads() {
    let file = rscleaner_rules::load::load_file(&fixture("valid/minimal.yaml")).expect("valid");
    assert_eq!(file.schema, 1);
    assert_eq!(file.rules.len(), 1);
    assert_eq!(file.rules[0].id.as_str(), "test.rule");
}

#[test]
fn bad_yaml_reports_file_line_and_column() {
    let errors =
        rscleaner_rules::load::load_file(&fixture("invalid/bad-yaml.yaml")).expect_err("must fail");
    assert_eq!(errors[0].line, 3);
    assert!(errors[0].column >= 1);
    assert_eq!(errors[0].code.as_str(), "E_YAML");
}

#[test]
fn unknown_field_reports_its_position() {
    let errors = rscleaner_rules::load::load_file(&fixture("invalid/unknown-field.yaml"))
        .expect_err("must fail");
    assert!(
        errors
            .iter()
            .any(|error| error.code.as_str() == "E_UNKNOWN_FIELD")
    );
}

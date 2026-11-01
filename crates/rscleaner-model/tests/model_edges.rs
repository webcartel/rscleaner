use rscleaner_model::{ByteSize, RuleId, Target};

#[test]
fn target_kind_defaults_to_path() {
    let target: Target = serde_json::from_str(r#"{"path":"C:/cache","mode":"contents"}"#)
        .expect("target with default kind");
    assert!(matches!(target, Target::Path { .. }));
}

#[test]
fn byte_size_parses_binary_suffixes() {
    let size: ByteSize = serde_json::from_str("\"50MB\"").expect("size");
    assert_eq!(size.0, 50 * 1024 * 1024);
}

#[test]
fn rule_id_rejects_uppercase_and_single_segments() {
    assert!("browsers".parse::<RuleId>().is_err());
    assert!("Browsers.chrome".parse::<RuleId>().is_err());
    assert!("browsers.chrome-cache".parse::<RuleId>().is_ok());
}

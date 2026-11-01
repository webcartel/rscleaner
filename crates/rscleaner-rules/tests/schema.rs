#[test]
fn generated_schema_has_the_rule_file_shape() {
    let schema = rscleaner_rules::schema::rule_file_schema();
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["rules"].is_object());
    assert!(schema["properties"]["schema"].is_object());
}

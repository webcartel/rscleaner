use rscleaner_model::RuleFile;
use serde_json::Value;

pub fn rule_file_schema() -> Value {
    let schema = schemars::schema_for!(RuleFile);
    serde_json::to_value(schema).unwrap_or(Value::Null)
}

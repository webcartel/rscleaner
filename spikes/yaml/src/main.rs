use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RuleFile {
    pub schema: u32,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Rule {
    pub id: String,
    pub name: String,
}

#[derive(Debug)]
pub struct ParseError {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

#[cfg(feature = "norway")]
pub fn parse_rule_file(path: &str) -> Result<RuleFile, ParseError> {
    let text = std::fs::read_to_string(path).map_err(|error| ParseError {
        file: path.to_string(),
        line: 1,
        column: 1,
        message: error.to_string(),
    })?;
    serde_norway::from_str(&text).map_err(|error| {
        let location = error.location();
        ParseError {
            file: path.to_string(),
            line: location.as_ref().map(|value| value.line()).unwrap_or(1),
            column: location.as_ref().map(|value| value.column()).unwrap_or(1),
            message: error.to_string(),
        }
    })
}

#[cfg(all(feature = "ng", not(feature = "norway")))]
pub fn parse_rule_file(path: &str) -> Result<RuleFile, ParseError> {
    let text = std::fs::read_to_string(path).map_err(|error| ParseError {
        file: path.to_string(),
        line: 1,
        column: 1,
        message: error.to_string(),
    })?;
    serde_yaml_ng::from_str(&text).map_err(|error| {
        let location = error.location();
        ParseError {
            file: path.to_string(),
            line: location.as_ref().map(|value| value.line()).unwrap_or(1),
            column: location.as_ref().map(|value| value.column()).unwrap_or(1),
            message: error.to_string(),
        }
    })
}

fn main() {
    if std::env::args().any(|argument| argument == "--schema") {
        match serde_json::to_string_pretty(&schemars::schema_for!(RuleFile)) {
            Ok(schema) => println!("{schema}"),
            Err(error) => {
                eprintln!("schema failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }

    println!("rscleaner-spike-yaml");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_yaml_loads() {
        let file = parse_rule_file("testdata/valid.yaml").expect("valid file");
        assert_eq!(file.schema, 1);
        assert_eq!(file.rules.len(), 1);
        assert_eq!(file.rules[0].id, "test.rule");
    }

    #[test]
    fn invalid_yaml_reports_line_and_column() {
        let err = parse_rule_file("testdata/invalid.yaml").expect_err("must fail");
        assert_eq!(err.line, 3);
        assert!(err.column >= 1);
    }

    #[test]
    fn schema_has_a_rules_property() {
        let schema = schemars::schema_for!(RuleFile);
        let json = serde_json::to_value(schema).expect("schema json");
        assert!(json["properties"]["rules"].is_object());
    }

    #[test]
    fn committed_schema_is_valid_json() {
        let text = std::fs::read_to_string("rule.schema.json").expect("schema file");
        let json: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert!(json["properties"]["rules"].is_object());
    }
}

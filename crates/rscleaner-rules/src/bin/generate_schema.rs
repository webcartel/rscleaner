use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let mut check = false;
    let mut path = PathBuf::from("schema/rule.schema.json");
    let mut args = std::env::args().skip(1);

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--check" => check = true,
            "--path" => {
                path = PathBuf::from(
                    args.next()
                        .ok_or_else(|| "missing value for --path".to_string())?,
                );
            }
            "--help" | "-h" => {
                println!("generate_schema [--check] [--path PATH]");
                return Ok(());
            }
            other if !other.starts_with('-') => path = PathBuf::from(other),
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }

    let schema = rscleaner_rules::schema::rule_file_schema();
    let rendered = format!("{}\n", serde_json::to_string_pretty(&schema)?);

    if check {
        let current = fs::read_to_string(&path)?;
        if current != rendered {
            eprintln!("schema is stale: {}", path.display());
            std::process::exit(1);
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, rendered)?;
    println!("wrote {}", path.display());
    Ok(())
}

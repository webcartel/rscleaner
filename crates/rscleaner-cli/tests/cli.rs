use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn version_prints_product_and_platform() {
    Command::cargo_bin("rscleaner")
        .expect("bin")
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("RSCleaner"))
        .stdout(predicate::str::contains(if cfg!(windows) {
            "windows"
        } else {
            "linux"
        }));
}

#[test]
fn invalid_language_exits_two() {
    Command::cargo_bin("rscleaner")
        .expect("bin")
        .args(["--lang", "de", "version"])
        .assert()
        .code(2);
}

#[test]
fn list_json_contains_a_builtin_rule() {
    Command::cargo_bin("rscleaner")
        .expect("bin")
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("browsers.chrome.cache"));
}

#[test]
fn config_path_prints_config_file() {
    Command::cargo_bin("rscleaner")
        .expect("bin")
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.yaml"));
}

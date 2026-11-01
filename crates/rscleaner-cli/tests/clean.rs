#![cfg(windows)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

struct Fixture {
    dir: tempfile::TempDir,
    tree: PathBuf,
    rules: PathBuf,
    config: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap_or_else(|_| panic!("temp"));
    let tree = dir.path().join("tree");
    std::fs::create_dir_all(&tree).unwrap_or_else(|_| panic!("tree"));
    std::fs::write(tree.join("a.tmp"), [0u8; 10]).unwrap_or_else(|_| panic!("file"));
    std::fs::write(tree.join("b.tmp"), [0u8; 5]).unwrap_or_else(|_| panic!("file"));

    let rules = dir.path().join("rules");
    std::fs::create_dir_all(&rules).unwrap_or_else(|_| panic!("rules"));
    let platform = if cfg!(windows) { "windows" } else { "linux" };
    let path = tree.to_string_lossy().replace('\\', "/");
    let yaml = format!(
        "schema: 1\nrules:\n  - id: test.temp\n    name: Test temp\n    category: test\n    risk: safe\n    platforms: [{platform}]\n    default_enabled: true\n    targets:\n      - path: '{path}'\n        mode: contents\n"
    );
    std::fs::write(rules.join("test.yaml"), yaml).unwrap_or_else(|_| panic!("rule"));

    let config = dir.path().join("config.yaml");
    std::fs::write(&config, "language: en\n").unwrap_or_else(|_| panic!("config"));

    Fixture {
        dir,
        tree,
        rules,
        config,
    }
}

fn command(fixture: &Fixture) -> Command {
    let mut command =
        Command::cargo_bin("rscleaner").unwrap_or_else(|_| panic!("the binary is built"));
    command
        .arg("--config")
        .arg(&fixture.config)
        .arg("--rules-dir")
        .arg(&fixture.rules);
    command
}

#[test]
fn scan_prints_the_plan_as_json() {
    let f = fixture();
    command(&f)
        .args(["scan", "--rule", "test.temp", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test.temp"))
        .stdout(predicate::str::contains("\"bytes_found\": 15"));
}

#[test]
fn clean_without_a_terminal_and_without_yes_exits_three() {
    let f = fixture();
    command(&f)
        .args(["clean", "--rule", "test.temp"])
        .write_stdin("")
        .assert()
        .code(3);
    assert!(f.tree.join("a.tmp").exists());
}

#[test]
fn a_dry_run_with_yes_keeps_every_file() {
    let f = fixture();
    command(&f)
        .args(["clean", "--rule", "test.temp", "--dry-run", "--yes"])
        .assert()
        .success();
    assert!(f.tree.join("a.tmp").exists());
    assert!(f.tree.join("b.tmp").exists());
}

#[test]
fn clean_with_yes_removes_the_files() {
    let f = fixture();
    command(&f)
        .args(["clean", "--rule", "test.temp", "--yes", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"bytes_freed\": 15"));
    assert!(!f.tree.join("a.tmp").exists());
    assert!(!f.tree.join("b.tmp").exists());
    assert!(Path::new(f.dir.path()).exists());
}

#[cfg(windows)]
#[test]
fn a_dangerous_rule_needs_the_flag() {
    let f = fixture();
    command(&f)
        .args(["clean", "--rule", "downloads.old-installers", "--yes"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--include-dangerous"));
}

#[test]
fn a_dry_run_leaves_the_tree_hash_unchanged() {
    use std::hash::{DefaultHasher, Hash, Hasher};
    use std::path::PathBuf;

    fn hash_tree(root: &Path) -> u64 {
        fn walk(dir: &Path, entries: &mut Vec<PathBuf>) {
            let Ok(read) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in read.flatten() {
                if entry.path().is_dir() {
                    walk(&entry.path(), entries);
                } else {
                    entries.push(entry.path());
                }
            }
        }
        let mut entries = Vec::new();
        walk(root, &mut entries);
        entries.sort();
        let mut hasher = DefaultHasher::new();
        for path in entries {
            path.strip_prefix(root).unwrap_or(&path).hash(&mut hasher);
            if let Ok(bytes) = std::fs::read(&path) {
                bytes.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    let f = fixture();
    let before = hash_tree(&f.tree);
    command(&f)
        .args(["clean", "--rule", "test.temp", "--dry-run", "--yes"])
        .assert()
        .success();
    let after = hash_tree(&f.tree);
    assert_eq!(before, after, "a dry run must not change the tree");
}

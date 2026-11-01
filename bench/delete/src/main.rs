use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rayon::prelude::*;
use serde_json::json;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    Std,
    Parallel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
}

pub fn remove_tree(root: &Path, strategy: Strategy) -> io::Result<Stats> {
    let stats = count_tree(root)?;
    match strategy {
        Strategy::Std => fs::remove_dir_all(root)?,
        Strategy::Parallel => remove_parallel(root)?,
    }
    Ok(stats)
}

fn count_tree(root: &Path) -> io::Result<Stats> {
    let mut stats = Stats {
        files: 0,
        dirs: 0,
        bytes: 0,
    };
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(io::Error::other)?;
        let file_type = entry.file_type();
        if file_type.is_dir() {
            stats.dirs += 1;
        } else if file_type.is_file() {
            stats.files += 1;
            stats.bytes += entry.metadata().map_err(io::Error::other)?.len();
        }
    }
    Ok(stats)
}

fn remove_parallel(root: &Path) -> io::Result<()> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_dir() {
            dirs.push(path);
        } else {
            files.push(path);
        }
    }

    for file in files {
        fs::remove_file(file)?;
    }

    dirs.par_iter().try_for_each(|dir| remove_parallel(dir))?;
    fs::remove_dir(root)
}

fn parse_args() -> Result<(PathBuf, Strategy), String> {
    let mut root = PathBuf::from("target/bench/delete");
    let mut strategy = Strategy::Std;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Err("usage".into()),
            "--root" => {
                root = PathBuf::from(
                    args.next()
                        .ok_or_else(|| "missing value for --root".to_string())?,
                );
            }
            "--strategy" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --strategy".to_string())?;
                strategy = match value.as_str() {
                    "std" => Strategy::Std,
                    "parallel" => Strategy::Parallel,
                    other => return Err(format!("unknown strategy: {other}")),
                };
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok((root, strategy))
}

fn print_usage() {
    println!("rscleaner-bench-delete --root PATH --strategy std|parallel");
}

fn main() {
    match parse_args() {
        Ok((root, strategy)) => {
            let started = Instant::now();
            match remove_tree(&root, strategy) {
                Ok(stats) => {
                    let output = json!({
                        "root": root.display().to_string(),
                        "strategy": match strategy { Strategy::Std => "std", Strategy::Parallel => "parallel" },
                        "files": stats.files,
                        "dirs": stats.dirs,
                        "bytes": stats.bytes,
                        "elapsed_ms": started.elapsed().as_millis(),
                    });
                    println!("{output}");
                }
                Err(error) => {
                    eprintln!("remove failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        Err(message) if message == "usage" => print_usage(),
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_strategy_deletes_a_tree() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("a/b")).expect("mkdir");
        std::fs::write(dir.path().join("a/b/file.txt"), b"x").expect("write");
        let stats = remove_tree(&dir.path().join("a"), Strategy::Std).expect("remove");
        assert_eq!(stats.files, 1);
        assert!(!dir.path().join("a").exists());
    }

    #[test]
    fn parallel_strategy_deletes_a_tree() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("a/b/c")).expect("mkdir");
        std::fs::write(dir.path().join("a/b/c/file.txt"), b"x").expect("write");
        std::fs::write(dir.path().join("a/file.txt"), b"y").expect("write");
        let stats = remove_tree(&dir.path().join("a"), Strategy::Parallel).expect("remove");
        assert_eq!(stats.files, 2);
        assert!(!dir.path().join("a").exists());
    }
}

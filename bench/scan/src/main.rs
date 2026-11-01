use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::json;
use sysinfo::{Pid, ProcessesToUpdate, System};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanStats {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
}

pub fn scan_tree(root: &Path) -> io::Result<ScanStats> {
    let mut stats = ScanStats {
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

fn peak_rss_kb() -> u64 {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system
        .process(Pid::from_u32(std::process::id()))
        .map(|process| process.memory() / 1024)
        .unwrap_or(0)
}

fn parse_args() -> Result<(PathBuf, usize), String> {
    let mut root = std::env::current_dir().map_err(|error| error.to_string())?;
    let mut threads = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
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
            "--threads" => {
                threads = args
                    .next()
                    .ok_or_else(|| "missing value for --threads".to_string())?
                    .parse()
                    .map_err(|_| "invalid --threads".to_string())?;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok((root, threads))
}

fn print_usage() {
    println!("rscleaner-bench-scan --root PATH [--threads N]");
}

fn main() {
    match parse_args() {
        Ok((root, threads)) => {
            let started = Instant::now();
            match scan_tree(&root) {
                Ok(stats) => {
                    let elapsed_ms = started.elapsed().as_millis();
                    let output = json!({
                        "root": root.display().to_string(),
                        "threads": threads,
                        "files": stats.files,
                        "dirs": stats.dirs,
                        "bytes": stats.bytes,
                        "elapsed_ms": elapsed_ms,
                        "peak_rss_kb": peak_rss_kb(),
                    });
                    println!("{output}");
                }
                Err(error) => {
                    eprintln!("scan failed: {error}");
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
    fn scan_counts_files_dirs_and_bytes() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("a/b")).expect("mkdir");
        std::fs::write(dir.path().join("a/one.bin"), b"1234").expect("write");
        std::fs::write(dir.path().join("a/b/two.bin"), b"12").expect("write");

        let stats = scan_tree(dir.path()).expect("scan");

        assert_eq!(stats.files, 2);
        assert_eq!(stats.dirs, 3);
        assert_eq!(stats.bytes, 6);
    }
}

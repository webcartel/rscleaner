use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct GenArgs {
    pub root: PathBuf,
    pub dirs: usize,
    pub files_per_dir: usize,
    pub depth: usize,
    pub size: u64,
    pub hardlinks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub dirs: usize,
    pub files: usize,
    pub bytes: u64,
}

pub fn generate(args: &GenArgs) -> io::Result<Stats> {
    fs::create_dir_all(&args.root)?;

    let dirs = args.dirs.max(1);
    let levels = args.depth.max(1).min(dirs);
    let top = (dirs / levels).max(1);

    let mut created: Vec<PathBuf> = Vec::with_capacity(dirs);
    for index in 0..top.min(dirs) {
        let path = args.root.join(format!("level-1-{index:04}"));
        fs::create_dir_all(&path)?;
        created.push(path);
    }

    let mut next = 1usize;
    while created.len() < dirs {
        let parent = created[next % created.len()].clone();
        let path = parent.join(format!("child-{next:04}"));
        fs::create_dir_all(&path)?;
        created.push(path);
        next += 1;
    }

    let mut files = 0usize;
    let mut bytes = 0u64;
    let mut first_file: Option<PathBuf> = None;

    for dir in &created {
        for file_index in 0..args.files_per_dir {
            let path = dir.join(format!("file-{file_index:04}.bin"));
            let file = File::create(&path)?;
            file.set_len(args.size)?;
            files += 1;
            bytes += args.size;
            if first_file.is_none() {
                first_file = Some(path);
            }
        }
    }

    if let Some(first) = first_file {
        for link_index in 0..args.hardlinks {
            let link = args.root.join(format!("hardlink-{link_index:04}.bin"));
            fs::hard_link(&first, &link)?;
        }
    }

    Ok(Stats {
        dirs: created.len(),
        files,
        bytes,
    })
}

fn parse_args() -> Result<GenArgs, String> {
    let mut args = std::env::args().skip(1);
    let mut result = GenArgs {
        root: PathBuf::from("target/bench/tree"),
        dirs: 1000,
        files_per_dir: 100,
        depth: 3,
        size: 4096,
        hardlinks: 0,
    };

    while let Some(arg) = args.next() {
        let value = |name: &str, args: &mut std::iter::Skip<std::env::Args>| {
            args.next()
                .ok_or_else(|| format!("missing value for {name}"))
        };
        match arg.as_str() {
            "--help" | "-h" => return Err("usage".into()),
            "--root" => result.root = PathBuf::from(value("--root", &mut args)?),
            "--dirs" => {
                result.dirs = value("--dirs", &mut args)?
                    .parse()
                    .map_err(|_| "invalid --dirs".to_string())?
            }
            "--files-per-dir" => {
                result.files_per_dir = value("--files-per-dir", &mut args)?
                    .parse()
                    .map_err(|_| "invalid --files-per-dir".to_string())?
            }
            "--depth" => {
                result.depth = value("--depth", &mut args)?
                    .parse()
                    .map_err(|_| "invalid --depth".to_string())?
            }
            "--size" => {
                result.size = value("--size", &mut args)?
                    .parse()
                    .map_err(|_| "invalid --size".to_string())?
            }
            "--hardlinks" => {
                result.hardlinks = value("--hardlinks", &mut args)?
                    .parse()
                    .map_err(|_| "invalid --hardlinks".to_string())?
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok(result)
}

fn print_usage() {
    println!(
        "rscleaner-bench-gen --root PATH --dirs N --files-per-dir N --depth N --size BYTES --hardlinks N"
    );
}

fn main() {
    match parse_args() {
        Ok(args) => match generate(&args) {
            Ok(stats) => {
                println!(
                    "root={} dirs={} files={} bytes={}",
                    args.root.display(),
                    stats.dirs,
                    stats.files,
                    stats.bytes
                );
            }
            Err(error) => {
                eprintln!("generate failed: {error}");
                std::process::exit(1);
            }
        },
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
    fn default_shape_matches_requested_counts() {
        let dir = tempfile::tempdir().expect("temp dir");
        let args = GenArgs {
            root: dir.path().to_path_buf(),
            dirs: 4,
            files_per_dir: 3,
            depth: 2,
            size: 1024,
            hardlinks: 0,
        };
        let stats = generate(&args).expect("generate");
        assert_eq!(stats.dirs, 4);
        assert_eq!(stats.files, 12);
    }

    #[test]
    fn files_are_placed_under_the_root() {
        let dir = tempfile::tempdir().expect("temp dir");
        let args = GenArgs {
            root: dir.path().join("tree"),
            dirs: 1,
            files_per_dir: 1,
            depth: 1,
            size: 8,
            hardlinks: 0,
        };
        let stats = generate(&args).expect("generate");
        assert_eq!(stats.dirs, 1);
        assert_eq!(stats.files, 1);
        assert!(dir.path().join("tree").is_dir());
    }
}

# RSCleaner

RSCleaner frees space on Windows 10 and Ubuntu 24.04 by rules written in YAML.
It scans the disk, shows what it found, and removes only what you approve.

## Status

RSCleaner is in early development. The first public release is planned as
`1.0.0`; pre-release builds are `0.x`.

## Supported platforms

- Windows 10 22H2 (build 19045): CLI, TUI, GUI, registry cleanup, space analysis.
- Ubuntu 24.04 with WSL: CLI and TUI. GUI and other distributions are not
  supported in 1.0.

Windows 11 is expected to work but is not verified yet. Older Windows 10
builds, Fedora, Arch, and Linux desktop GUI are not supported in 1.0.

## Build

Requirements: Rust 1.99.1 or newer, Node.js 24 for the GUI, and WebView2 on
Windows.

```sh
cargo build --release
cargo run -p rscleaner-cli -- version
cargo run -p rscleaner-cli -- list
cargo run -p rscleaner-cli -- scan
cargo run -p rscleaner-cli -- clean --dry-run
cargo run -p rscleaner-cli -- rules validate rules
```

`scan` prints the plan and changes nothing. `clean` asks for confirmation and
removes the planned items; `--dry-run` takes the same code path without
removing anything. `--json` prints one document, `--events` prints ndjson
events, and `--report <file>` writes the report.

The GUI build will be documented after the M0 spike. The CLI and TUI do not
depend on WebView2.

## Safety

RSCleaner never deletes a path that has not passed the guard. Tests use
temporary directories only. The journal must open before a clean starts.

## License

MIT. See `LICENSE`.

# Changelog

All notable changes to RSCleaner are recorded here.

## 0.1.0 — 2026-11-01

### Added

- Cargo workspace and repository guardrails.
- Synthetic tree generator and scan/delete benchmarks.
- Tauri 2 + Vue 3 GUI shell with the RSCleaner design tokens.
- YAML parser spike with positional errors and generated JSON Schema.
- Trash and TypeScript binding spikes.
- Typed rule, config, profile, and error model (`schema: 1`).
- YAML loader with `file:line:column` diagnostics.
- Variable expansion, glob validation, risk inference, and 40+ invalid cases.
- Source merging and `default`, `dev`, and `aggressive` profiles.
- 32 Windows rules and the verified Ubuntu 24.04 WSL rule subset.
- Configuration load/save and product data paths.
- CLI commands: `version`, `list`, `rules validate`, `rules show`, `config path`.
- CI checks for schema freshness, translation keys, and the docs boundary.

### Decisions

- RSCleaner is a standalone product with no compatibility layer.
- Windows 10 22H2 and Ubuntu 24.04 WSL are the verified platforms.
- Rule and report schema 1; YAML rules/config, JSON reports/events.
- MIT license.

## 0.2.0-m2 — 2026-11-01

### Added

- `rscleaner-platform`: `Fs`, `Env`, `KnownFolders`, `Procs`, `Elevation`, and
  `SystemInfo` traits with Windows and Linux backends and in-memory fakes.
- File identity from the volume serial and file index on Windows, and from
  device and inode on Linux.
- `rscleaner-guard`: path normalization with variable expansion, extended-length
  Windows paths, `..` removal, trailing dot and space trimming, and case rules.
- Protection tables for both platforms, browser-profile allowlists, `VerifiedPath`,
  `Guard::verify`, `Guard::reverify`, and a `Remover` that accepts only a
  `VerifiedPath`.
- Junction and symlink handling: a link is removed, its target is never followed.
- 100 Windows and 100 Linux table cases, property tests, and a `cargo-fuzz`
  target for normalization.

### Fixed

- A description of a variable with a closing `}` consumed an extra byte and
  could split a multibyte character; the fuzzer found it on Linux.
- Risk inference now reads declared path components, so the answer does not
  depend on the machine's environment.
- `downloads.old-installers` is `dangerous`, because it targets a user-data
  directory.

### Decisions

- Raw filesystem calls stay in `rscleaner-platform`; the cleanup boundary is
  `Remover` plus `VerifiedPath`.
- Longest protection range wins; ties go to the stricter rule.
- A `contents` rule never traverses a link; a `self` rule may remove the link.

## Unreleased

- M3: scanner, executor, `Engine`, and CLI `scan`/`clean`.

## 0.3.0-m3 — 2026-11-01

### Added

- `rscleaner-scan`: rule targets become a plan. `contents`, `self`, and `files`
  modes, variable and glob expansion in the declared path, `include`/`exclude`
  masks, `min_age_days`, `min_size`, and hard links counted once.
- `rscleaner-report`: report schema 1 with `bytes_found`, `bytes_freed`,
  `bytes_trashed`, per-rule statuses, and skip and failure entries; an
  append-only jsonl journal; a generated `schema/report.schema.json`.
- `rscleaner-exec`: dry run, trash, journal before removal, `requires_closed`
  and `requires_admin` skips, `InUse` handling with `--on-reboot` deferral on
  Windows, and `dangerous` rules always going to the trash.
- `rscleaner-app`: the `Engine` facade with `scan`, `clean`, `plan_items`, a
  bounded event channel, a cancel token, and the last four plans.
- CLI `scan` and `clean` with `--profile`, `--rule`, `--dry-run`, `--yes`,
  `--trash`/`--no-trash`, `--include-dangerous`, `--on-reboot`, `--report`,
  `--json`, and `--events`; exit codes 0, 1, 2, 3, 4, and 130.
- Command targets: no shell, an absolute path or a bare name resolved only in
  trusted system directories, a cleared environment, a timeout, cancellation,
  and the output in the journal.
- New runtime error codes: `E_IO`, `E_IN_USE`, `E_COMMAND`, `E_GUARD`,
  `E_JOURNAL`, and `E_CANCELLED`.

### Fixed

- Missing or denied paths no longer collapse into `NotFound`: a permission
  error is reported as `AccessDenied`.
- Windows identity reads use a zero-access handle, which cut the 100,000-file
  scan from 73 s to 3.5 s.

### Decisions

- Glob components in a `path` target are expanded against the filesystem; a
  literal suffix after a glob that does not exist is dropped, not reported.
- The planner reads the declared path components for risk; the executor
  re-verifies every item immediately before removal.
- Linux end-to-end CLI tests wait for M5, where the verified Ubuntu rules and
  the `/tmp` filter rules exist.

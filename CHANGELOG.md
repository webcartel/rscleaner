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

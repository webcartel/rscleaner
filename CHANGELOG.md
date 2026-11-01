# Changelog

All notable changes to RSCleaner are recorded here.

## Unreleased

### Added

- Cargo workspace and repository guardrails.
- Synthetic tree generator and scan/delete benchmarks.
- Tauri 2 + Vue 3 GUI shell with the RSCleaner design tokens.
- YAML parser spike with positional errors and generated JSON Schema.
- Trash and TypeScript binding spikes.

### Decisions

- RSCleaner is a standalone product with no compatibility layer.
- Windows 10 22H2 and Ubuntu 24.04 WSL are the verified platforms.
- Rule and report schema 1; YAML rules/config, JSON reports/events.
- MIT license.

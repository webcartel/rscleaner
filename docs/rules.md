# Built-in rules

RSCleaner 1.0 ships 32 Windows rules and a verified Ubuntu 24.04 WSL subset.
Every rule is YAML with `schema: 1`; the format is described in
[format.md](format.md).

## Windows

All Windows rules are verified against the Windows 10 22H2 baseline. If an
application is not installed, its rule simply finds nothing.

| Category | Rules |
| --- | --- |
| Browsers | `browsers.chrome.cache`, `browsers.edge.cache`, `browsers.firefox.cache` |
| Apps | `apps.discord.cache`, `apps.notion.cache`, `apps.obsidian.cache`, `apps.slack.cache`, `apps.steam.cache`, `apps.telegram.cache`, `apps.vscode.cache` |
| Development | `dev.docker-prune`, `dev.go-build-cache`, `dev.gradle-caches`, `dev.npm-cache`, `dev.nuget-cache`, `dev.pip-cache`, `dev.pnpm-store`, `dev.yarn-cache` |
| Downloads | `downloads.old-installers` |
| System | `system.cbs-logs`, `system.component-cleanup`, `system.crash-dumps`, `system.delivery-optimization`, `system.minidumps`, `system.recycle-bin`, `system.temp`, `system.thumbnail-cache`, `system.wer-machine`, `system.wer-user`, `system.windows-old`, `system.windows-temp`, `system.windows-update-cache` |

## Ubuntu 24.04 WSL

Only paths confirmed on Ubuntu 24.04 WSL are shipped.

| Category | Rules |
| --- | --- |
| System | `linux.apt.clean`, `linux.tmp.old`, `linux.var-tmp.old`, `linux.logs.rotated`, `linux.trash.contents` |
| Development | `linux.cache.go-build`, `linux.cache.cargo` |

Docker, Flatpak, desktop applications, pip, npm, Yarn, and Gradle caches are
not shipped in 1.0 because their paths were not confirmed on the test system.

## Profiles

- `default` selects rules with `default_enabled: true`.
- `dev` adds rules tagged `dev`.
- `aggressive` selects every rule except `dangerous`.

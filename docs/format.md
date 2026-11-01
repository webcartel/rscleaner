# Rule format

RSCleaner rules are YAML files with `schema: 1`.

```yaml
schema: 1
rules:
  - id: browsers.chrome.cache
    name: Chrome cache
    category: browsers
    risk: safe
    default_enabled: true
    platforms: [windows]
    requires_closed: [chrome.exe]
    targets:
      - path: '${LOCALAPPDATA}/Google/Chrome/User Data/*/Cache'
        mode: contents
```

## Rule fields

| Field | Required | Meaning |
| --- | --- | --- |
| `id` | yes | Stable lowercase dotted id, for example `browsers.chrome.cache`. |
| `name` | yes | String or `{en, ru}` map. |
| `description` | no | String or `{en, ru}` map. |
| `category` | yes | Group such as `browsers`, `apps`, `dev`, `system`, `downloads`. |
| `risk` | yes | `safe`, `moderate`, or `dangerous`. |
| `default_enabled` | no | Include in the `default` profile. |
| `tags` | no | Profile tags such as `dev`. |
| `platforms` | yes | `windows` and/or `linux`. |
| `when` | no | `distro`, `command_exists`, or `path_exists` conditions. |
| `requires_admin` | no | Administrator or root is required. |
| `requires_closed` | no | Process names that must not be running. |
| `os_min_build` | no | Minimum Windows build. |
| `trash` | no | Send matches to the platform trash. |
| `targets` | yes | One or more targets. |

## Targets

### `path`

Deletes the contents, the matched object, or matching files.

| Field | Meaning |
| --- | --- |
| `path` | Path with variables and globs. |
| `mode` | `contents`, `self`, or `files`. Defaults to `contents`. |
| `include` / `exclude` | Name masks. |
| `min_age_days` | Only objects older than this. |
| `min_size` | Only files at least this size, for example `50MB`. |

### `find`

Finds a marker directory such as `node_modules` under the search roots.
`has_sibling` requires a nearby file, `max_depth` limits the walk, and
`min_age_days` skips recently used projects.

### `command`

Runs a program without a shell. User rules that use commands require
`allow_user_commands: true` in `config.yaml`.

### `registry`

Built-in Windows rules only. User rules cannot use registry targets.

## Variables

Windows: `${LOCALAPPDATA}`, `${APPDATA}`, `${PROGRAMDATA}`, `${USERPROFILE}`,
`${SYSTEMROOT}`, `${SYSTEMDRIVE}`, `${TEMP}`, `${PROGRAMFILES}`,
`${PROGRAMFILES(X86)}`.

Linux: `${HOME}`, `${XDG_CACHE_HOME}`, `${XDG_CONFIG_HOME}`,
`${XDG_DATA_HOME}`, `${XDG_STATE_HOME}`, `${TMPDIR}`.

Both: `~`, `${known:Downloads}`, `${known:Documents}`, `${known:Desktop}`,
`${known:Pictures}`, `${known:Videos}`, `${known:Music}`.

An unknown variable is an error. A known variable that is not set on this
machine makes the rule not applicable.

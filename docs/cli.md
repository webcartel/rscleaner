# Command line

RSCleaner installs one binary, `rscleaner`. The terminal interface arrives in
a later milestone; the current commands are the foundation layer.

```sh
rscleaner version
rscleaner list [--profile default|dev|aggressive] [--category NAME] [--platform windows|linux] [--json]
rscleaner rules validate FILE_OR_DIR...
rscleaner rules show RULE_ID
rscleaner config path [--json]
```

## Common options

`--lang en|ru` selects the interface language. `--config PATH` selects a
configuration file. `--rules-dir PATH` adds user rule files. `--json` prints
one JSON document where the command supports it.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | Internal error |
| 2 | Invalid flags, configuration, or rules |
| 3 | Confirmation is impossible without a terminal |
| 4 | The run finished with paths left behind |
| 130 | Interrupted by the user |

Codes 3, 4, and 130 are reserved for `clean` and arrive with the executor.

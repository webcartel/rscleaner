#!/usr/bin/env bash
set -euo pipefail

allowed=(
  "docs/format.md"
  "docs/rules.md"
  "docs/registry.md"
  "docs/space.md"
  "docs/cli.md"
)

status=0
while IFS= read -r file; do
  case "$file" in
    AGENTS.md|agent-docs/*|.agents/*|.superpowers/*)
      echo "agent-only file is tracked: $file" >&2
      status=1
      ;;
    docs/*)
      found=0
      for candidate in "${allowed[@]}"; do
        if [ "$file" = "$candidate" ]; then
          found=1
          break
        fi
      done
      if [ "$found" -ne 1 ]; then
        echo "unexpected file in docs/: $file" >&2
        status=1
      fi
      ;;
  esac
done < <(git ls-files)

exit "$status"

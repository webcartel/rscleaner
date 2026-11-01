#!/usr/bin/env bash
set -euo pipefail

PYTHON="${PYTHON:-python3}"
if ! command -v "$PYTHON" >/dev/null 2>&1; then
  PYTHON=python
fi

"$PYTHON" - <<'PY'
import json
import pathlib
import sys

root = pathlib.Path("crates/rscleaner-i18n/locales")
paths = [root / "en.json", root / "ru.json"]
sets = []

def no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate key: {key}")
        result[key] = value
    return result

for path in paths:
    with path.open(encoding="utf-8") as handle:
        data = json.load(handle, object_pairs_hook=no_duplicates)
    if not isinstance(data, dict):
        raise SystemExit(f"{path}: expected a JSON object")
    sets.append(set(data))

if sets[0] != sets[1]:
    missing_ru = sorted(sets[0] - sets[1])
    missing_en = sorted(sets[1] - sets[0])
    print(f"missing in ru: {missing_ru}", file=sys.stderr)
    print(f"missing in en: {missing_en}", file=sys.stderr)
    raise SystemExit(1)
PY

#!/bin/sh
# Native release QA in a disposable prefix; never installs globally or publishes.
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "$FERRULE_ROOT"
mkdir -p .work
RELEASE_OUTPUT=$(mktemp -d "$FERRULE_ROOT/.work/release-XXXXXX")
if [ -x "$FERRULE_ROOT/../.venv/bin/python" ]; then
  RELEASE_PYTHON="$FERRULE_ROOT/../.venv/bin/python"
else
  RELEASE_PYTHON=python3
fi
"$RELEASE_PYTHON" scripts/release.py build --output "$RELEASE_OUTPUT"
for RELEASE_ARCHIVE in "$RELEASE_OUTPUT"/*.tar.gz "$RELEASE_OUTPUT"/*.zip; do
  if [ -f "$RELEASE_ARCHIVE" ]; then
    "$RELEASE_PYTHON" scripts/release.py check "$RELEASE_ARCHIVE"
  fi
done
echo "PASS: native release package in $RELEASE_OUTPUT (not published)"

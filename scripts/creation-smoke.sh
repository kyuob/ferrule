#!/bin/sh
# Actual generator and final directory installation; no app installation or launch.
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "$FERRULE_ROOT"
cargo build --locked --quiet
mkdir -p .work
CREATION_ROOT=$(mktemp -d "$FERRULE_ROOT/.work/creation-XXXXXX")
FERRULE="$FERRULE_ROOT/target/debug/ferrule"
echo "Creation artifacts: $CREATION_ROOT"
"$FERRULE" new creation_app --directory "$CREATION_ROOT"
test -f "$CREATION_ROOT/creation_app/ferrule.toml"
test -f "$CREATION_ROOT/creation_app/pubspec.yaml"
test -f "$CREATION_ROOT/creation_app/rust/Cargo.toml"
test -f "$CREATION_ROOT/creation_app/lib/main.dart"
test -d "$CREATION_ROOT/creation_app/ios"
test -d "$CREATION_ROOT/creation_app/android"
test -z "$(find "$CREATION_ROOT" -maxdepth 1 -name '.ferrule-create-*' -print)"
"$FERRULE" add network --dry-run --project "$CREATION_ROOT/creation_app" > "$CREATION_ROOT/module-preview.json"
if "$FERRULE" new creation_app --directory "$CREATION_ROOT" > "$CREATION_ROOT/retry.log" 2>&1; then
  echo 'Existing destination was unexpectedly accepted' >&2
  exit 1
fi
test -f "$CREATION_ROOT/creation_app/ferrule.toml"
echo 'PASS: real core generation, complete directory install, ownership preview and existing-destination refusal'

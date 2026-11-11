#!/bin/sh
# Real generation/bridge tests; does not install apps or claim platform key verification.
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "$FERRULE_ROOT"
cargo build --locked --quiet
mkdir -p .work
SECURITY_SMOKE_ROOT=$(mktemp -d "$FERRULE_ROOT/.work/security-XXXXXX")
echo "Security artifacts: $SECURITY_SMOKE_ROOT"
FERRULE="$FERRULE_ROOT/target/debug/ferrule"
"$FERRULE" new private_app --modules security --directory "$SECURITY_SMOKE_ROOT"
"$FERRULE" sync --project "$SECURITY_SMOKE_ROOT/private_app"
cd "$SECURITY_SMOKE_ROOT/private_app"
sh "$FERRULE_ROOT/scripts/security-app-check.sh" "$PWD"
"$FERRULE" add data --dry-run > "$SECURITY_SMOKE_ROOT/data-preview.json"
"$FERRULE" add data
"$FERRULE" add ble
"$FERRULE" sync
sh "$FERRULE_ROOT/scripts/security-app-check.sh" "$PWD"
echo "PASS: optional security generation, real bridge crypto/file persistence and security/data/BLE composition (fake platform key storage only)"

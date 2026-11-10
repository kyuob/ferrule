#!/bin/sh
# Real network/toolchain check; not part of the offline CLI test suite.
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "$FERRULE_ROOT"
cargo build --locked --quiet
mkdir -p .work
SMOKE_ROOT=$(mktemp -d "$FERRULE_ROOT/.work/smoke-XXXXXX")
echo "Smoke artifacts: $SMOKE_ROOT"
FERRULE="$FERRULE_ROOT/target/debug/ferrule"
"$FERRULE" new smoke_app --modules core --directory "$SMOKE_ROOT"
"$FERRULE" sync --project "$SMOKE_ROOT/smoke_app"
cd "$SMOKE_ROOT/smoke_app"
flutter analyze
cargo test --manifest-path rust/Cargo.toml --quiet
"$FERRULE" add ble --dry-run > "$SMOKE_ROOT/add-preview.json"
test ! -e .ferrule.lock
test ! -e lib/features/ble/ble_page.dart
"$FERRULE" add ble
"$FERRULE" sync
flutter analyze
cargo test --manifest-path rust/Cargo.toml --quiet
"$FERRULE" add ble
"$FERRULE" new ble_app --modules ble --directory "$SMOKE_ROOT"
"$FERRULE" sync --project "$SMOKE_ROOT/ble_app"
cd "$SMOKE_ROOT/ble_app"
flutter analyze
cargo test --manifest-path rust/Cargo.toml --quiet
echo "PASS: core generation, read-only add preview, structural BLE addition, repeated addition and new --modules ble"

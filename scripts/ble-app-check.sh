#!/bin/sh
# Inspect a synchronized BLE app with fake radio/GATT input, never hardware.
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "${1:?Usage: sh scripts/ble-app-check.sh GENERATED_APP}"
test -f ferrule.toml
if [ -f test/security_bridge_test.dart ]; then
  sh "$FERRULE_ROOT/scripts/security-app-check.sh" "$PWD"
else
  flutter analyze
  cargo test --manifest-path rust/Cargo.toml --quiet
  flutter test test/ble_session_test.dart test/ble_page_test.dart
fi
echo 'PASS: BLE session and widget checks (fake radio/GATT, no hardware claim)'

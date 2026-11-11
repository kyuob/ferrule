#!/bin/sh
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "$FERRULE_ROOT"
cargo build --locked --quiet
mkdir -p .work
GATT_SMOKE_ROOT=$(mktemp -d "$FERRULE_ROOT/.work/gatt-XXXXXX")
echo "GATT artifacts: $GATT_SMOKE_ROOT"
FERRULE="$FERRULE_ROOT/target/debug/ferrule"
"$FERRULE" new sensor_app --modules ble --directory "$GATT_SMOKE_ROOT"
"$FERRULE" sync --project "$GATT_SMOKE_ROOT/sensor_app"
cd "$GATT_SMOKE_ROOT/sensor_app"
sh "$FERRULE_ROOT/scripts/ble-app-check.sh" "$PWD"
"$FERRULE" add data security --dry-run > "$GATT_SMOKE_ROOT/add-preview.json"
"$FERRULE" add data security
"$FERRULE" sync
sh "$FERRULE_ROOT/scripts/ble-app-check.sh" "$PWD"
echo 'PASS: BLE0.2 generation, fake GATT lifecycle and BLE/data/security composition'

#!/bin/sh
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "$FERRULE_ROOT"
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked

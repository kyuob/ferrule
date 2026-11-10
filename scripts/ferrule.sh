#!/bin/sh
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
# Keep the caller's working directory for `new`, `add` and `sync`.
exec cargo run --quiet --locked --manifest-path "$FERRULE_ROOT/Cargo.toml" -- "$@"

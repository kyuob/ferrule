#!/bin/sh
# Source from a project-local script after setting FERRULE_ROOT.
if [ -x "$FERRULE_ROOT/.tools/cargo/bin/cargo" ]; then
  CARGO_HOME="$FERRULE_ROOT/.tools/cargo"
  RUSTUP_HOME="$FERRULE_ROOT/.tools/rustup"
  PATH="$CARGO_HOME/bin:$PATH"
  export CARGO_HOME RUSTUP_HOME PATH
fi

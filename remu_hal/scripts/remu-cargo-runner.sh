#!/usr/bin/env bash
set -euo pipefail
elf="${1:?remu-cargo-runner.sh <elf-path>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ws="$(cd "$here/../.." && pwd)"
cd "$ws"
# Embedded `cargo run` sets CARGO_TARGET_DIR (e.g. target/app); that env is inherited by this
# runner. Host `remu_cli` / `xtask` must use the workspace default target dir — otherwise their
# artifacts live under target/app and `just clean-app` wipes them, forcing a full rebuild.
unset CARGO_TARGET_DIR
cmd="$(cargo run -p xtask --manifest-path "$ws/Cargo.toml" -- print run-remu "$elf")"
eval "$cmd"

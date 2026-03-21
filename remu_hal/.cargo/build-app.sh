#!/usr/bin/env sh
# Expand ISA shorthand to full triple and run cargo build + objdump.
# Runs from remu_hal/ so its .cargo/config.toml (RISC-V targets) is used.
# Usage: build-app.sh <app> <target> <root>
set -e
triple="$2"
case "$triple" in *-*) ;; *) triple="${triple}-unknown-none-elf";; esac
export CARGO_TARGET_DIR="$3/target/app"
cd "$3/remu_hal"
cargo build --release -p "remu_app_$1" --target "$triple" -Z build-std=core --manifest-path "$3/Cargo.toml"
cargo objdump --release -p "remu_app_$1" --target "$triple" --bin "remu_app_$1" -Z build-std=core --manifest-path "$3/Cargo.toml" -- -d > "$CARGO_TARGET_DIR/$triple/release/remu_app_$1.disasm"

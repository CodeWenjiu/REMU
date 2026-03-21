#!/usr/bin/env sh
# Linker wrapper for RISC-V remu-app: run rust-lld.
# Disassembly via cargo objdump in build-app.sh.
set -e

REAL_LINKER="${REMU_LINKER:-rust-lld}"
if ! command -v "$REAL_LINKER" >/dev/null 2>&1; then
    REAL_LINKER="ld.lld"
fi
if ! command -v "$REAL_LINKER" >/dev/null 2>&1; then
    echo "remu linker-wrapper: neither rust-lld nor ld.lld found, using rust-lld (may fail)" >&2
    REAL_LINKER="rust-lld"
fi

exec "$REAL_LINKER" "$@"

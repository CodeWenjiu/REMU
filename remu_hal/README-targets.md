# RISC-V targets

Built-in triples: `riscv32i-unknown-none-elf`, `riscv32im-unknown-none-elf`, `riscv32imac-unknown-none-elf`.

## Zve shorthand (`riscv32im_zve32x_zvl128b`)

Handled entirely in **xtask** (no Zve JSON in-tree):

- **`--target riscv32im-unknown-none-elf`**
- **`CARGO_TARGET_RISCV32IM_UNKNOWN_NONE_ELF_RUSTFLAGS=-C target-feature=+zve32x,+zvl128b`** (merged with `.cargo/config.toml` for that triple)
- **`REMU_ISA=riscv32im_zve32x_zvl128b`** on `print run-app` so `run-remu` matches remu (inner dir is still `riscv32im-unknown-none-elf/`).
- **`CARGO_TARGET_DIR`** uses **`target/app_zve32x`** instead of **`target/app`** so Zve and plain RV32IM builds do not overwrite each other.

Optional **`remu_hal/<name>.json`** targets still work; printed commands add **`-Z json-target-spec`**.

### `xtask print` (stdout; `eval` at repo root)

```bash
eval "$(cargo run -p xtask -- print run-app <name> riscv32im_zve32x_zvl128b)"
eval "$(cargo run -p xtask -- print build-app <name> riscv32im_zve32x_zvl128b)"
```

`just build-app` / `just run-app` wrap the above. `clean-app` removes `target/app` and `target/app_zve32x`.

`remu-cargo-runner.sh` **unsets `CARGO_TARGET_DIR`** before building/running host `remu_cli`: otherwise that variable leaks from embedded `cargo run` and host artifacts would live under `target/app*`, so `clean-app` would delete them and force a full `remu_cli` rebuild.

### Manual Zve (no xtask)

```bash
export REMU_ISA=riscv32im_zve32x_zvl128b
export CARGO_TARGET_RISCV32IM_UNKNOWN_NONE_ELF_RUSTFLAGS='-C target-feature=+zve32x,+zvl128b'
export CARGO_TARGET_DIR=target/app_zve32x   # optional; avoids clobbering target/app
cargo run -p remu_app_<name> --target riscv32im-unknown-none-elf --release -Z build-std=core,alloc
```

Use **`-Z build-std=core,alloc`** for these apps.

# RISC-V targets

HAL overview: [README.md](README.md) · [README_zh.md](README_zh.md)

Built-in triples: `riscv32i-unknown-none-elf`, `riscv32im-unknown-none-elf`, `riscv32imac-unknown-none-elf`.

## Named extension shorthands (xtask, winnow)

Multi-segment ISA strings (anything that is **not** a single standard RISC-V letter in the triple) are parsed in **`remu_hal/xtask/src/isa_shorthand.rs`**: base `riscv32i` / `riscv32im` / `riscv32imac`, then zero or more `_`-separated **named** segments. Each segment is registered there (longest match first in the parser). The resolved **`CargoTarget`** is checked against **`remu_types::isa::IsaSpec`** so `REMU_ISA` / `--isa` stays consistent with the rest of the workspace.

Adding another named extension: extend **`NamedExtension`**, **`named_extension_segment`**, and the match in **`CargoTarget::try_from_parsed`** (and add **`ExtensionSpec`** in `remu_types` if the simulator should understand it).

### wjCus0 (`riscv32im_wjCus0`, `riscv32i_wjCus0`)

- **`--target`** is the usual base triple (`riscv32im-unknown-none-elf`, etc.).
- **`print run-app`** sets **`REMU_ISA`** to the full shorthand (e.g. `riscv32im_wjCus0`) so `remu_cli --isa` matches the ELF. You can still use **`EXISA0=1`** with a plain base target if you prefer; it appends `_wjCus0` in `print run-remu` when **`REMU_ISA`** is not already suffixed.

### Zve shorthand (`riscv32im_zve32x_zvl128b`, `riscv32i_zve32x_zvl128b`)

Handled entirely in **xtask** (no Zve JSON in-tree):

- **`--target`** is the base triple (`riscv32im-unknown-none-elf` or `riscv32i-unknown-none-elf`).
- **`CARGO_TARGET_RISCV32IM_UNKNOWN_NONE_ELF_RUSTFLAGS`** or **`CARGO_TARGET_RISCV32I_UNKNOWN_NONE_ELF_RUSTFLAGS`** is set to **`-C target-feature=+zve32x,+zvl128b`** (merged with any existing value for that env var).
- **`REMU_ISA`** is set to e.g. **`riscv32im_zve32x_zvl128b`** on `print run-app` so `run-remu` matches remu.
- **`CARGO_TARGET_DIR`** uses **`target/app_zve32x`** instead of **`target/app`** so Zve and plain RV32IM builds do not overwrite each other.

**Note:** Combining **`_zve32x_zvl128b`** with **`_wjCus0`** in one shorthand is rejected by xtask until remu supports that ISA matrix.

Optional **`remu_hal/<name>.json`** targets still work; printed commands add **`-Z json-target-spec`**.

### `xtask print` (stdout; `eval` at repo root)

```bash
eval "$(cargo run -p xtask -- print run-app <name> riscv32im_zve32x_zvl128b)"
eval "$(cargo run -p xtask -- print run-app <name> riscv32im_wjCus0)"
eval "$(cargo run -p xtask -- print build-app <name> riscv32im_zve32x_zvl128b)"
eval "$(cargo run -p xtask -- print build-app <name> riscv32im_wjCus0)"
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

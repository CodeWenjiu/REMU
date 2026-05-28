---
name: isa-variant
description: Add a new ISA variant (e.g., RV64I, RV32IMF) to the project. Covers updating the for_each_isa! table, dispatch macros, and IsaKind enums across remu_isa, remu_harness, remu_simulator_nzea, and remu_boot.
---

## Files to modify (in order)

### 1. `remu_isa/src/isa/extension_enum.rs` — the table

Add a row to `for_each_isa!`:
```
$cb!(RV64I, u64, -, -, NoV, -, 0x80...0100, "rv64i", i, none, R);
```
Columns: `Name, XLEN_type, has_M(+/-), has_F(+/-), VConfig_type, has_WJ(+/-), MISA_value, ISA_string, base_arch(i/im/ia/imac), ext_spec(none/wj/zve), platforms(R/N/RN)`

The `gen_isa_type!` macro auto-generates the struct + `RvIsa` impl. No manual code needed.

Update `gen_isa_type!` 8 arms if the new ISA uses a new combination of `+`/`-` (e.g., adding an F-enabled variant when none existed before requires 4 new arms).

### 2. Platform IsaKind enums

**For remu** (`remu_harness/src/isa_dispatch.rs`):
- Add variant to `RemuIsaKind` enum
- Add match arm in `from_isa_spec_or_panic`

**For nzea** (`remu_simulator_nzea/src/supported_isa.rs`):
- Add variant to `NzeaIsaKind` enum (if nzea supports it)
- Add match arm in `try_from_isa_spec`

### 3. Dispatch macros (`remu_boot/src/lib.rs`)

Add match arms to `dispatch_remu!` and/or `dispatch_nzea!`:
```
RemuIsaKind::Rv64I => $runner.run_with_config::<$Config<RV64I>>($opt, $irq),
```

### 4. Import the new ISA type

In `remu_boot/src/lib.rs`, add to the `use` block:
```rust
use remu_isa::isa::extension_enum::{..., RV64I};
```

## Platform support

When adding a new ISA, decide which platforms support it:
- `RN` in the table → both remu and nzea dispatch need arms
- `R` → only remu dispatch needs an arm
- `N` → only nzea dispatch needs an arm

## Verification

```bash
cargo check --workspace --exclude remu_hal --exclude remu_app_hello_world --exclude remu_app_collection --exclude remu_app_mnist
```

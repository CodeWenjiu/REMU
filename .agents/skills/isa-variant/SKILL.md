---
name: isa-variant
description: Add a new ISA variant (e.g., RV64I, RV32IMF) to the project. Covers updating the for_each_isa! table, dispatch macros, and IsaKind enums across remu_isa, remu_harness, remu_simulator_nzea, and remu_boot.
---

## Files to modify (in order)

### 1. `remu_isa/src/isa/extension_enum.rs` — the table

Add a row to `for_each_isa!`. One row is the complete definition of one ISA: the
same fields generate the `Isa` enum variant (value-level attributes), the
`RV64IM`-style marker struct and its `RvIsa` impl (type-level state layout).

```
RV64IM {
    xlen: 64, fpr: (), v: $crate::isa::extension_v::NoV,
    misa: 0x0000_1100, isa_str: "rv64im",
},
```

Fields:
- `xlen`: `32` | `64` — drives `type XLEN` (via `xlen_type!`) and `Isa::xlen()`.
- `fpr`: FPR state type — `()` (no F, zero-sized) or `FprRegs` (F present).
  `HAS_F` is derived from this type by `fpr_present!`, so the constant can
  never disagree with the state layout.
- `v`: V config type — `NoV` or `Zve32xZvl128b`; supplies `VrState` and `VLENB`.
- `misa`: low 32 bits of the `misa` CSR (MXL lives in bits [63:62] and is
  derived from `xlen`, so RV64 rows read `0x0000_....`).
- `isa_str`: canonical ISA string (`Isa::isa_str()` / `ISA_STR`).
- `HAS_M` / `MISA` / `ISA_STR` are all derived from the row (`Isa::has_m()` reads
  the `misa` M bit), so no value is written twice.

No manual `impl RvIsa` is ever needed. Both `xlen_type!` and `fpr_present!` have
an arm per value; a new XLEN or a new FPR-state kind needs a new arm there.

### 2. Platform IsaKind enums (the support matrix lives in the backends)

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

Which backend accepts which ISA is defined **in the backends**, not in the ISA
table: `RemuIsaKind::from_isa_spec_or_panic` and `NzeaIsaKind::try_from_isa_spec`
are the support matrices. Adding a variant to the table does not make any
backend accept it — decide per backend and add the enum variant + match arm
there, plus the matching `dispatch_*!` arm in `remu_boot/src/lib.rs`.
For nzea also extend the `NzeaIsa` impl (`nzea_ffi.rs`) with the
`NZEA_ISA_STR` the nzea RTL dump expects.

## Verification

```bash
cargo check --workspace --exclude remu_hal --exclude remu_app_hello_world --exclude remu_app_collection --exclude remu_app_mnist
```

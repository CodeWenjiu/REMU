---
name: flow-files
description: Create or update flow/ directories (command.rs, option.rs, generic.rs) for crates that need data-flow conventions. Use when a crate needs runtime commands, initialization options, or compile-time generic configuration.
---

## Convention (from AGENTS.md)

When a crate needs runtime init, compile-time generics, or operation commands, group them in `src/flow/`:

```
src/flow/
  mod.rs       → remu_macro::mod_flat!(command, option, generic);
  command.rs   → Runtime operation commands (clap Subcommand)
  option.rs    → Runtime initialization config (clap Args)
  generic.rs   → Compile-time generic type config (traits, PhantomData)
```

The parent `lib.rs` declares it:
```rust
remu_macro::mod_pub_flat!(flow);
```

## Creating files

### command.rs template

```rust
use crate::flow::option::SomeSubOption;

#[derive(Debug, clap::Subcommand)]
pub enum XxxCmd {
    /// Sub-command description
    SubCmd {
        #[command(flatten)]
        subcmd: SubCmd,
    },
}
```

The enum wraps sub-layer commands (e.g., `StateCmd` wraps `BusCmd` and `RegCmd`).

### option.rs template

```rust
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct XxxOption {
    /// Description of this field
    #[arg(long, default_value = "...")]
    pub field: Type,

    #[command(flatten)]
    pub sub_option: SubOption,
}
```

The struct wraps sub-layer options via `#[command(flatten)]`.

### generic.rs template

```rust
use std::marker::PhantomData;

pub trait XxxPolicy {
    type ISA: RvIsa;
    type Observer: BusObserver;
}

pub struct XxxProfile<ISA>(PhantomData<ISA>);

impl<ISA: RvIsa> XxxPolicy for XxxProfile<ISA> {
    type ISA = ISA;
    type Observer = SomeObserver;
}
```

### flow/mod.rs

```rust
remu_macro::mod_flat!(command, option, generic);
```

If the crate has no generics, remove `generic` from the list. If no commands, remove `command`.

## Pattern recurses downward

Sub-modules can also have `flow/`:
- `remu_state/src/bus/flow/` — `BusCmd`, `BusOption`
- `remu_state/src/reg/flow/` — `RegCmd`, `RegOption`

## What NOT to put in flow/

- `error.rs` — errors flow UP (response channel), not DOWN. Keep at crate root.
- Implementation logic — `flow/` is only for data that passes from upper to lower layers.

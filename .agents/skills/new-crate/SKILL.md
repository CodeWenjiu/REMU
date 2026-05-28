---
name: new-crate
description: Scaffold a new Rust crate in the remu workspace following all conventions. Use when creating a new library crate, simulator backend, or utility crate.
---

## Steps

### 1. Create directory and Cargo.toml

```
remu_new_crate/
├── Cargo.toml
└── src/
    └── lib.rs
```

`Cargo.toml` template:
```toml
[package]
name = "remu_new_crate"
version = "0.1.0"
edition = "2024"

[dependencies]
remu_macro = { path = "../remu_macro" }

[lints]
workspace = true
```

- Edition must be `2024`
- Always include `remu_macro` if the crate has any modules
- Add other dependencies as needed (path-based for internal crates, workspace deps for external)

### 2. Create lib.rs with module declarations

```rust
remu_macro::mod_flat!(/* same-dir files */);
remu_macro::mod_pub!(/* sub-dir modules */);
```

See `module-setup` skill for the full rules.

### 3. Add prelude (if crate exports types)

Create `src/prelude.rs`:
```rust
//! Public API surface of `remu_new_crate`. Import via `use remu_new_crate::prelude::*;`.

pub use crate::some_module::SomeType;
```

In `lib.rs`:
```rust
remu_macro::mod_pub_flat!(prelude);
```

### 4. Add flow directory (if crate has commands/options/generics)

See `flow-files` skill for details.

### 5. Register in workspace

Add to root `Cargo.toml` members list:
```toml
members = [
    ...
    "remu_new_crate",
]
```

### 6. Initial check

```bash
cargo check -p remu_new_crate
```

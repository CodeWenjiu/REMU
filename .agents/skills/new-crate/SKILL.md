---
name: new-crate
description: Scaffold a new Rust crate in the remu workspace following all conventions. Use when creating a new library crate, simulator backend, or utility crate.
---

## Steps

### 0. Choose the zone

Placement follows role and reuse scope (see also `AGENTS.md`):

| Crate | Zone |
|---|---|
| Part of the simulator stack | root `remu_*`, flat |
| Runnable program | `remu_app/bins/<name>/` |
| Platform facility shared by apps | `remu_app/hal/<crate>/` |
| Reusable app-side library (two or more consumers, or app-independent) | `remu_app/libs/<crate>/` |
| Private helper/codegen of one app | nested inside that app's directory |

Nesting beyond these cases is a violation.

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
- Adjust the `remu_macro` relative path for the chosen depth (e.g. `../../../remu_macro` from `remu_app/libs/<crate>`)
- Add other dependencies as needed (path-based for internal crates, workspace deps for external)

### 2. Create lib.rs with module declarations

```rust
remu_macro::mod_prv!(/* private, same-dir files */);
remu_macro::mod_pub!(/* public sub-modules */);
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
remu_macro::mod_pub!(prelude);
```

### 4. Add flow directory (if crate has commands/options/generics)

See `flow-files` skill for details.

### 5. Register in workspace

Add to the root `Cargo.toml` `members` list:

- root crate: `"remu_new_crate"`;
- direct child of `remu_app/`: already covered by the `remu_app/*` glob — no edit;
- under a group directory (`remu_app/libs`, `remu_app/bins/<project>`): add an explicit entry — member globs for paths under an `exclude`d group directory are dropped. The group directory itself must be listed in `exclude` (it is not a package).

### 6. Initial check

```bash
cargo check -p remu_new_crate
```

---
name: module-setup
description: Create or reorganize Rust modules following the remu Module Declaration Constitution. Use when adding a new .rs file, moving files, or fixing bare mod violations.
---

## Module Declaration Constitution (from AGENTS.md)

Every crate MUST declare its modules exclusively through `remu_macro` macros. Manual `mod` / `pub mod` / `pub use` for module plumbing is forbidden.

| Directory shape | Macro | Generated code |
|---|---|---|
| `src/X.rs` (same-dir file) | `mod_flat!(X)` | `mod X; pub use X::*;` |
| `src/X/mod.rs` (sub-dir) | `mod_pub!(X)` | `pub mod X;` |
| `src/X.rs` (file, needs path access) | `mod_pub_flat!(X)` | `pub mod X; pub use X::*;` |

## Rules

1. **Single-call-per-type**: Each macro (`mod_flat!`, `mod_pub!`, `mod_pub_flat!`) appears at most once per file. Merge same-type calls.
2. **Different types can coexist**: One `mod_flat!` + one `mod_pub!` is fine.
3. **Inline modules exempt**: `mod func3 { ... }`, `mod tests { ... }` don't need macros.
4. **`as` alias exception**: `pub use LongName as Short;` is OK after `mod_pub!`.
5. **Selective re-exports**: Control visibility inside the module with `pub`/`pub(crate)`; `mod_flat!` exports only `pub` items.

## Workflow

### Adding a new same-directory file

1. Create `src/new_file.rs`
2. Find the existing `mod_flat!` call in `lib.rs` (or `mod.rs` for sub-modules)
3. Add the new name to the list
4. If no `mod_flat!` exists yet, create one: `remu_macro::mod_flat!(new_file);`

### Adding a new sub-directory module

1. Create `src/new_module/mod.rs`
2. Find the existing `mod_pub!` call
3. Add the new name
4. If no `mod_pub!` exists, create one: `remu_macro::mod_pub!(new_module);`

### Adding a file that needs both path access and flat export (e.g., prelude)

1. Create `src/prelude.rs`
2. Use `remu_macro::mod_pub_flat!(prelude);` — this replaces both `pub mod prelude;` and `pub use prelude::*;`

### Common mistakes to catch

- Using `mod_pub!` for same-directory files → should be `mod_flat!`
- Bare `mod X;` without macro → violation
- Separate `pub use X::*;` when `mod_flat!` already does it → redundant
- Two `mod_flat!` calls instead of one merged call → violation of single-call-per-type

## Exception

`remu_macro/src/lib.rs` uses bare `mod module; mod pattern;` — this is the ONLY allowed exception (bootstrap problem: the macros are defined inside those modules).

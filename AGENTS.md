# Repository Guidelines

## Project Structure & Module Organization
This repository is a Rust workspace (Edition 2024, nightly toolchain) centered on a RISC-V simulator/debugger stack.

- `remu_cli`, `remu_debugger`: interactive CLI and command handling.
- `remu_simulator`: simulator abstraction and backends (`simulators/remu`, `simulators/spike`, `simulators/nzea`).
- `remu_isa`: ISA type definitions (RvIsa trait, register types, Xlen, extension configs).
- `remu_state`, `remu_types`, `remu_fmt`, `remu_macro`, `remu_logger`: shared state, utility types, formatting/parsing, macros, logging.
- `remu_hal`, `remu_hal/xtask`: embedded HAL + task helpers used by app build/run flows.
- `remu_app/*`: sample apps (`hello_world`, `mnist`, `collection`).

Note: `remu` is intended to be used inside the parent `chip-dev` checkout with submodules.

### Module Declaration Constitution (MUST follow)

Every crate MUST declare its modules exclusively through `remu_macro` macros. **Manual `mod` / `pub mod` / `pub use` for module plumbing is forbidden** — the macros are the single source of truth for how modules are wired into the crate.

| Directory shape | Macro | Generated code | When |
|---|---|---|---|
| `src/X.rs` (same-dir file) | `remu_macro::mod_flat!(X);` | `mod X; pub use X::*;` | Single or multiple `.rs` files directly in `src/` |
| `src/X/mod.rs` (sub-dir) | `remu_macro::mod_pub!(X);` | `pub mod X;` | Module is a directory with its own nested structure |
| `src/X.rs` (file, but needs path access) | `remu_macro::mod_pub_flat!(X);` | `pub mod X; pub use X::*;` | Single file that must be both publicly accessible by path AND flattened (e.g., `prelude.rs`) |

```rust
// ✅ CORRECT — same-directory files use mod_flat!
remu_macro::mod_flat!(error, func, option, generic, run_state);

// ✅ CORRECT — sub-directories use mod_pub!
remu_macro::mod_pub!(reg, bus);

// ❌ WRONG — manual mod, mod_pub! for flat files, bare pub mod all violate the rules above
```

**Rationale**: `mod_flat!` communicates "this file's public API is part of the crate's flat namespace"; `mod_pub!` communicates "this is a sub-module with its own hierarchy". When every crate follows this convention, readers instantly know where to find code without guessing whether a module was manually wired or macro-generated.

> See `.agents/skills/module-setup/` for step-by-step workflows and common mistakes.

**Single-call-per-type rule**: Each macro (`mod_flat!`, `mod_pub!`, or `mod_pub_flat!`) MUST appear at most once per file. Merge all same-directory files into one `mod_flat!` call, and all sub-directory modules into one `mod_pub!` call. Different macro types may coexist (e.g., one `mod_flat!` + one `mod_pub_flat!` is fine).

**Inline modules are exempt**: `mod func3 { ... }`, `mod tests { ... }`, and similar inline module blocks that do NOT reference external files are not subject to these rules — only file-based module declarations are.

**`as` alias exception**: When a module needs a public alias (`pub use LongName as Short;`), keep the `pub use` line after `mod_pub!` — this is the one case where a manual `pub use` is necessary because `mod_pub!` cannot express aliases.

**Selective re-exports**: Control visibility *inside* the module — mark items `pub` only if they belong in the crate's public API, `pub(crate)` if they're shared within the crate but should not be re-exported, and private otherwise. Then `mod_flat!` naturally exports exactly the right set. Do NOT add manual `pub use` lines after `mod_flat!` (they are redundant).

**`#[macro_export]` macro rules**: A macro annotated with `#[macro_export]` MUST be defined and consumed in the same Rust source file. Never `use` a `#[macro_export]` macro across modules within the same crate — this triggers Rust future-compatibility errors and defeats the purpose of the module convention. External crates import normally via `use crate_name::macro_name;`.

**`prelude` module convention**: Crates define `src/prelude.rs` and declare it with `remu_macro::mod_pub_flat!(prelude);`. This makes prelude both path-accessible (`remu_xxx::prelude::*`) and flattened at the crate root. No separate `pub use crate::prelude::*;` line is needed.

**Exception — `remu_macro` bootstrap**: `remu_macro/src/lib.rs` uses bare `mod module; mod pattern;` because `mod_flat!` / `mod_pub!` are defined *inside* those modules. This is the **only** crate allowed to use bare `mod`, and the reason must be documented with a comment.

### Data-Flow File Conventions (SHOULD follow)

When a crate needs to define its own runtime initialization, compile-time generics, or operation commands, group them in a `src/flow/` subdirectory:

```
src/flow/
  mod.rs         → remu_macro::mod_flat!(command, option, generic);
  command.rs     → Runtime operation commands
  option.rs      → Runtime initialization config
  generic.rs     → Compile-time generic type configuration
```

The parent `lib.rs` declares it with `remu_macro::mod_pub_flat!(flow);` so items are both path-accessible and flat.

Rules:
- Create only the files needed. Skip `generic.rs` or `command.rs` if not applicable.
- The pattern recurses downward: `src/flow/`, `src/bus/flow/`, `src/reg/flow/`.
- `generic.rs` replaces the old `policy.rs` naming.

> See `.agents/skills/flow-files/` for templates and detailed conventions.

## Design Decisions & Patterns (SHOULD understand)

This section records smaller architectural choices that guide day-to-day implementation. Unlike the Module Declaration Constitution, these are not enforced by macros — they are conventions to follow when writing new code.

### Comment philosophy

Do not add verbose comments on business logic. Comments are only warranted on complex generics or `unsafe` blocks (with `// Safety:` justification).

### Performance-first architecture

`State` lives under `Simulator`, not `Debugger`. The performance bottleneck is instruction execution, so State must be owned by the simulator to enable unchecked memory access, aggressive inlining, and zero-cost generics on the hot path.

### Tracer frontend/backend decoupling

The CLI defines a concrete `Tracer` trait implementation (the "frontend" — how data is displayed). The simulator and harness layers only see a `TracerDyn` ( `Rc<RefCell<dyn Tracer>>` ) and call it when they have information to output. The frontend decides display format; the backend decides what and when to emit.

## Build, Test, and Development Commands

**All commands MUST run inside the Nix dev shell** (`nix develop` or via `direnv allow`). The flake provides required toolchains (Rust nightly, mold, verilator, clang, cmake, etc.) and library paths (zlib, openssl) that the linker needs. Never run `cargo` directly outside the shell.

Use `just` recipes for day-to-day work:

- `just build`: build `remu_cli` in debug mode.
- `just dev -- <args>`: run debug CLI with backtraces.
- `just run -- <args>`: run release CLI with backtraces.
- `just build-app <app> [target]`: build embedded app via `xtask`.
- `just run-app <app> [target]`: build and run app on selected platform/ISA target.
- `just clean-app` / `just clean-all`: remove app artifacts / full workspace artifacts.

Direct Cargo examples: `cargo test --workspace`, `cargo run -p remu_cli --release -- --help`.

## Coding Style & Naming Conventions
- Follow Rust defaults: 4-space indentation, `snake_case` for functions/modules, `PascalCase` for types/traits, `SCREAMING_SNAKE_CASE` for constants.
- Keep crates and modules focused by layer (CLI, simulator, state, HAL).
- Prefer `cargo fmt --all` and `cargo clippy --workspace --all-targets` before submitting.
- Workspace lints are enabled via `lints.workspace = true`; treat warnings as actionable.

## Testing Guidelines
- Primary test command: `cargo test --workspace`.
- Unit tests are in-module (`#[test]`), e.g. in `remu_hal/xtask` and parser-related crates.
- Add tests with each behavior change, especially ISA parsing/target resolution and simulator correctness paths.
- No repository-wide coverage threshold is currently enforced; prioritize meaningful execution-path coverage.

## Commit & Pull Request Guidelines
- Recent history follows Conventional-Commit-like prefixes: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, and scoped forms like `feat(app): ...`.
- Keep commits small and single-purpose; use imperative summaries.
- PRs should include: problem statement, key changes, validation commands run, and related issue links.
- For CLI/output changes, include example commands and representative output snippets.

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

### Build Script Rules (MUST follow)

1. **Cross-crate logic goes through `[build-dependencies]`.** When a library crate (e.g. `nzea`) owns linker flags or build logic that the final binary (e.g. `cli`) must emit, the library exposes a **normal Rust function** (not a build.rs side-effect), and `cli` depends on it via `[build-dependencies]` and calls it. Never try to smuggle data through `cargo:rustc-link-arg` from a library's build.rs.

2. **`cargo:rustc-link-arg` from a library crate's build.rs only applies to that library's compilation, never to the final binary.** Linker flags for the binary MUST come from the binary crate's own build.rs (or a function called from it).

### Module Declaration Constitution (MUST follow)

Every crate MUST declare its modules through `remu_macro` macros. **Bare `mod` / `pub mod` for file-based module plumbing is forbidden** — the macros are the single source of truth for how modules are wired into the crate.

#### The two macros

| Macro | Usage | Expands to |
|-------|-------|------------|
| `mod_prv!(X, Y);` | Crate-private modules | `mod X; mod Y;` |
| `mod_pub!(X, Y);` | Public sub-modules | `pub mod X; pub mod Y;` |
| `mod_pub!(crate, X, Y);` | Crate-visible modules | `pub(crate) mod X; pub(crate) mod Y;` |
| `mod_pub!(super, X, Y);` | Parent-visible modules | `pub(super) mod X; pub(super) mod Y;` |

```rust
// ✅ CORRECT
remu_macro::mod_prv!(error, compound_command);
remu_macro::mod_pub!(reg, bus);
remu_macro::mod_pub!(crate, flow);
remu_macro::mod_pub!(super, helpers);

// Explicit re-exports (AFTER macros):
pub use wordlen::{Xlen, MachineWord};
pub use flow::command::{Command, DebuggerCommand};

// ❌ WRONG
mod internal;            // bare mod without macro
pub use internal::*;     // wildcard re-export
```

**Rationale**: The macro communicates intent ("private implementation" vs "public API"). Visibility is controlled at the module level with Rust's native `pub` / `pub(crate)` / `pub(super)` keywords. Explicit `pub use` lines make the crate's public API auditable — every exported symbol is visible in lib.rs.

> See `.agents/skills/module-setup/` for step-by-step workflows and common mistakes.

#### Visibility principle (MUST follow)

1. **Module is the minimum unit of visibility control.** Prefer `mod_pub!(crate, X)` over individually re-exporting symbols from `X`. If most of a module's `pub` items are re-exported at the parent level, the module itself should be visible at that level.

2. **Minimize symbol scope.** Use the most restrictive visibility possible:
   - `pub` only for true public API (used by external crates)
   - `pub(crate)` for crate-internal sharing
   - `pub(super)` for parent-module-only sharing
   - Default (private) otherwise

3. **No wildcard re-exports.** `pub use X::*;` is forbidden — it defeats the purpose of explicit visibility control and causes unnecessary recompilation cascades.

**Single-call-per-type rule**: Each macro MUST appear at most once per file. Merge all same-type modules into one call.

**Inline modules are exempt**: `mod tests { ... }` and similar inline blocks are not subject to these rules.

**`as` alias exception**: Manual `pub use LongName as Short;` is allowed after macros for aliasing.

**`#[macro_export]` macro rules**: A macro annotated with `#[macro_export]` MUST be defined and consumed in the same Rust source file. External crates import normally via `use crate_name::macro_name;`.

**`prelude` module convention**: Declare with `mod_pub!(prelude);`. Use explicit `pub use crate::module::Item;` inside (never `pub use *;`). External crates import via `use remu_xxx::prelude::*;`.

**Exception — `remu_macro` bootstrap**: Uses bare `mod module; mod pattern;` because the macros are defined inside those modules. This is the only crate allowed to use bare `mod`.

### Data-Flow File Conventions (SHOULD follow)

When a crate needs to define its own runtime initialization, compile-time generics, or operation commands, group them in a `src/flow/` subdirectory:

```
src/flow/
  mod.rs         → remu_macro::mod_prv!(command, option, generic);
  command.rs     → Runtime operation commands
  option.rs      → Runtime initialization config
  generic.rs     → Compile-time generic type configuration
```

The parent `lib.rs` declares it with `remu_macro::mod_pub!(crate, flow);`.

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

### Error handling: detect-and-consume (MUST follow)

When an error is detected deep in the call stack and the correct response is to print diagnostic information to the user, the detailed error data MUST be **consumed at the point of printing** and not propagated upward in full.

- **Detection layer** (harness / simulator): Print the detailed diagnostic (e.g., difftest mismatch register dump), then return a bare, data-free error variant upward.
- **Intermediate layers**: Pass the error up without re-printing.
- **Entry layer** (CLI / `main`): Add context to stderr (e.g., `"startup execution error: ..."`) but when converting to `anyhow::Error`, carry only the fact of failure (e.g., `"startup failed"`).

This prevents error details from being printed twice — once by the layer that detected the error and again by the layer that reports the failure. The principle is: **where an error is printed, it is consumed** (哪里处理哪里消耗).

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

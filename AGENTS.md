# Repository Guidelines

## Project Structure & Module Organization
This repository is a Rust workspace (Edition 2024, nightly toolchain) centered on a RISC-V simulator/debugger stack.

- `remu_cli`, `remu_debugger`: interactive CLI and command handling.
- `remu_simulator`: simulator abstraction and backends (`simulators/remu`, `simulators/spike`, `simulators/nzea`).
- `remu_state`, `remu_types`, `remu_fmt`, `remu_macro`, `remu_logger`: shared state, ISA/types, formatting/parsing, macros, logging.
- `remu_hal`, `remu_hal/xtask`: embedded HAL + task helpers used by app build/run flows.
- `remu_app/*`: sample apps (`hello_world`, `mnist`, `collection`).

Note: `remu` is intended to be used inside the parent `chip-dev` checkout with submodules.

## Build, Test, and Development Commands
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

# remu

**English** | [简体中文](README_zh.md)

**remu** is a **RISC-V** execution and debugging stack written in **Rust**. It pairs an interactive front-end with pluggable CPU simulators, optional **differential testing** against a reference model, and a path to co-simulate **RTL** (e.g. via Verilator).

> **Submodule layout:** this repository is a **git submodule** inside the larger **[chip-dev](https://github.com/CodeWenjiu/chip-dev)** project. Because of **workspace / path dependencies**, it is **not fully self-contained** yet. To **build and run tests locally**, clone **[chip-dev](https://github.com/CodeWenjiu/chip-dev) in full** (including submodules, e.g. `git clone --recursive https://github.com/CodeWenjiu/chip-dev.git`) and work from that tree—not from a standalone checkout of `remu` alone.

---

## Acknowledgments

This project is **inspired by and draws on ideas from [NEMU](https://github.com/NJU-ProjectN/nemu)** (Nanjing University’s educational ISA simulator). The overall workflow—stepping, state inspection, and bring-up patterns familiar from the NEMU ecosystem—heavily influenced remu’s design. **Thank you to the NEMU / Project-N authors and community** for the excellent reference implementation and documentation.

---

## Performance

The interpreter core has been **optimized for steady-state execution** (decode/dispatch, memory access patterns, and hot-path layout). On the same workload, remu reaches roughly **an order of magnitude higher throughput than the author’s earlier C-based NEMU-style implementation**.

**Workload:** [Abstract Machine (AM)](https://github.com/NJU-ProjectN/abstract-machine) **microbench**, **`ref` scale**, ISA **`riscv32im`** (RV32 + **M** extension). All scores below use the same binary and host environment.

| Simulator   | microbench-ref score |
|------------|----------------------:|
| **remu**   | **5503**              |
| Spike      | 11183                 |
| QEMU       | 23468                 |

*Scores are higher-is-better for this benchmark; numbers are a single reference run and will vary by CPU, compiler, and build flags.*

**Reference host** (where the table above was measured; yours will differ):

| | |
|--|--|
| **OS** | Linux **x86_64**, **WSL2** (kernel `6.6.87.2-microsoft-standard-WSL2`) |
| **CPU** | **Intel Core i5-13600KF** (guest view: **10 cores / 20 threads**, 1 socket) |
| **RAM** | **~32 GiB** |

**Speed vs remu (same benchmark):**

| vs **remu** | Relative |
|------------|----------|
| Spike      | ~2.0×    |
| QEMU       | ~4.3×    |
| Author’s C NEMU (prior work) | remu ~**10×** faster |

**Reproduce** (inside a full **[chip-dev](https://github.com/CodeWenjiu/chip-dev)** checkout, from the **`am-zig/`** directory):

```bash
cd am-zig
BATCH=true just run <platform> riscv32 im am-microbench ref
```

Replace **`<platform>`** with `remu`, `spike`, `qemu`, etc. **`BATCH=true`** runs the workload non-interactively (same idea as in remu’s own `run-app` tooling).

---

## Architecture: decoupled front-end & pluggable backends

remu **separates the debugger / CLI (front-end) from the execution engine (back-end)**:

- **Multiple simulators** can be plugged in as backends (e.g. the built-in Rust ISA model, Spike, or other adapters you add).
- **Differential testing (difftest)** is integrated: the DUT and a **reference model** advance in lockstep; register and memory state are compared to catch semantic mismatches early.
- **Hardware / RTL** can participate through a suitable adapter—for example a **Verilator**-based cycle model—so you can debug HDL against the same front-end and difftest infrastructure as the software simulators.

This layout keeps the UI and debugging workflow stable while you swap or combine **fast functional models**, **cycle-accurate RTL**, and **golden references**.

---

## Supported ISAs

**RV32** only today (`--isa …`, default **`riscv32i`**):

| `--isa` example | M | Vector (Zve32x, VLEN 128) |
|-----------------|---|---------------------------|
| `riscv32i` / `rv32i` | | |
| `riscv32im` / `rv32im` | ✓ | |
| `rv32i_zve32x_zvl128b` | | ✓ |
| `rv32im_zve32x_zvl128b` | ✓ | ✓ |

---

## Repository layout (overview)

| Area | Role |
|------|------|
| `remu_cli` / `remu_debugger` | Interactive shell and debugging commands |
| `remu_simulator` | Simulator abstraction and concrete backends (`remu`, Spike, …) |
| `remu_state`, `remu_types` | Architectural state, CSRs, ISA typing |
| `remu_hal`, `remu_app/*` | Embedded HAL (`riscv-rt`, `embedded-hal`, `embedded-io`, …) and runnable `no_std` apps — **[remu_hal/README.md](remu_hal/README.md)** · [中文](remu_hal/README_zh.md) |

---

## Environment & workflow (Nix, direnv, just)

The **supported developer environment is Nix-managed** via [`flake.nix`](flake.nix): **Rust nightly** (with `rust-src`, `clippy`, `rust-analyzer`, `llvm-tools-preview`), **RISC-V bare-metal targets** (`riscv32i` / `im` / `imac` `unknown-none-elf`), **Verilator**, **clang/libclang**, **mold**, and **`just`**.

### Nix + direnv

1. Install [Nix](https://nixos.org/download.html) with **flakes** enabled (`experimental-features = nix-command flakes` in `nix.conf`).
2. Install [direnv](https://direnv.net/) and **hook it into your shell** (bash/zsh/fish).
3. Optional: [nix-direnv](https://github.com/nix-community/nix-direnv) to cache the dev shell and speed up loads.
4. Clone the repo, `cd` into it, run **`direnv allow`** when prompted (`.envrc` uses **`use flake`** so entering the directory loads the dev shell).

**Without Nix:** you must supply a compatible **Rust nightly** (workspace uses **Edition 2024**), the same **RV32 bare-metal** targets, and host tools yourself—the flake is the reference setup.

### just

Day-to-day commands go through **[just](https://github.com/casey/just)** using the root [`justfile`](justfile) (available inside the Nix shell).

| Recipe | What it does |
|--------|----------------|
| `just` | List all recipes |
| `just build` | Debug build: `cargo build -p remu_cli` |
| `just run -- ARGS…` | **Release** `remu_cli` with `RUST_BACKTRACE=1` |
| `just dev -- ARGS…` | **Debug** `remu_cli` with `RUST_BACKTRACE=1` |
| `just build-app APP [TARGET]` | Build embedded crate `remu_app_{APP}` via **xtask** (default target `riscv32i`) |
| `just run-app APP [TARGET]` | Build + run that app under remu (see [`remu_hal/README-targets.md`](remu_hal/README-targets.md) for `TARGET` e.g. `riscv32im_zve32x_zvl128b`) |
| `just clean-app` | Remove `target/app` and `target/app_zve32x` |
| `just clean-all` | `cargo clean` |

Examples:

```bash
just run -- --help
just run-app hello_world
just run-app mnist riscv32im_zve32x_zvl128b
```

**Temporary env vars (`run-app` / embedded `cargo run`):** the app runner (`remu_hal/scripts/remu-cargo-runner.sh`) asks **xtask** to print a `remu_cli` command. Set options for that invocation by exporting variables **on the same line** as `just` (or in your shell) so they are visible when the runner runs:

| Variable | Effect |
|----------|--------|
| **`PLATFORM`** | `--platform …` for `remu_cli`: `remu` (default in CLI), `spike`, `nzea`, `none` |
| **`DIFFTEST`** | Enable difftest with reference model: `spike` or `remu` (omit / unset = **off**) |
| **`DEV`** | If set (any value), `print run-remu` uses **debug** `remu_cli` (`cargo run -p remu_cli` without `--release`). **Embedded `remu_app_*` stays `--release`** (`run-app` / `build-app` unchanged) |
| **`BATCH`** | If set (any value), adds `--batch --startup continue` for non-interactive runs |

Example: run **mnist** on **remu** with **Spike** as difftest reference:

```bash
PLATFORM=remu DIFFTEST=spike just run-app mnist riscv32im_zve32x_zvl128b
```

Other recipes (`look`, `step-sizes`, …) are for profiling / asm inspection—run **`just --list`**.

### Plain Cargo (inside the shell)

```bash
cargo build --release -p remu_cli
cargo run -p remu_cli --release -- …
```

---

## License

Component licenses may differ (e.g. vendored Spike carries its own `LICENSE`). See individual crates and third-party trees for details.

---

*This documentation was produced with the assistance of AI tools. It may contain errors or omissions; please verify critical details against the source code and your own testing.*

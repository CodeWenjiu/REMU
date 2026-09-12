# remu

**English** | [简体中文](README_zh.md)

**remu** is a **RISC-V** execution and debugging stack written in **Rust**. It pairs an interactive front-end with pluggable CPU simulators, optional **differential testing** against a reference model, and a **cycle-accurate RTL co-simulation** backend (nzea, driven through Verilator).

> **Submodule layout:** this repository is a **git submodule** inside the larger **[chip-dev](https://github.com/CodeWenjiu/chip-dev)** project. Because of **workspace / path dependencies**, it is **not fully self-contained** yet. To **build and run tests locally**, clone **[chip-dev](https://github.com/CodeWenjiu/chip-dev) in full** (including submodules, e.g. `git clone --recursive https://github.com/CodeWenjiu/chip-dev.git`) and work from that tree—not from a standalone checkout of `remu` alone.

---

## Acknowledgments

This project is **inspired by and draws on ideas from [NEMU](https://github.com/NJU-ProjectN/nemu)** (Nanjing University’s educational ISA simulator). The overall workflow—stepping, state inspection, and bring-up patterns familiar from the NEMU ecosystem—heavily influenced remu’s design. **Thank you to the NEMU / Project-N authors and community** for the excellent reference implementation and documentation.

---

## Performance

The interpreter core is optimized for steady-state execution: decode/dispatch is a single flattened jump table, the instruction cache aliases decoded state directly, the PC lives in registers across batch runs, and hot-path memory access is a software-TLB (addend) with no error construction on hit.

The built-in benchmark is `remu_app/microbench` (ported from the [AM microbench](https://github.com/NJU-ProjectN/am-kernels) suite), scored relative to a reference CPU. On the author's laptop (Core Ultra 5 125H):

| Simulator   | microbench `ref` score | guest wall time |
|-------------|-----------------------:|-----------------:|
| **remu**    | **~3800**              | ~4.5 s           |
| Spike       | ~3700 (with `--real-time-clint`) | ~7 s   |

> The two simulators are within ~10% of each other in raw host cycles. Older READMEs reported a ~2-3× "Spike advantage" — that was an artifact of Spike's default mtime advancing by **instruction count** rather than wall clock, which made its guest-reported times ~3.5× too fast. `just run-app … --platform spike` now passes `--real-time-clint` so guest times are comparable across platforms.

Scores vary with CPU frequency scaling (check your `scaling_governor`), compiler, and load; treat them as ballpark. For a frequency-independent comparison use `perf stat -e cycles` over the run.

**Reproduce** (from this repository):

```bash
just run-app microbench riscv32im --platform remu --app-args ref -- --platform remu --batch --startup continue
just run-app microbench riscv32im --platform spike --app-args ref
```

---

## Architecture: decoupled front-end & pluggable backends

remu **separates the debugger / CLI (front-end) from the execution engine (back-end)**:

- **Multiple simulators** plug in as backends: `remu` (built-in Rust ISA model), `spike` (vendored C++ reference), and `nzea` (Verilator RTL co-simulation).
- **Differential testing (difftest)** is integrated: the DUT and a **reference model** advance in lockstep; register and memory state are compared to catch semantic mismatches early (`--difftest remu` / `--difftest spike`).
- **Hardware / RTL** participates through `nzea`: a Verilator-generated cycle model of the custom core, communicating with the front-end over DPI. The same difftest and interactive front-end drives it.
- `--platform none` runs the front-end without a simulator (e.g. for debugging commands only).

This layout keeps the UI and debugging workflow stable while you swap or combine **fast functional models**, **cycle-accurate RTL**, and **golden references**.

---

## Supported ISAs

**RV32** (default `--isa riscv32i`):

| `--isa` example | M | Vector (Zve32x, VLEN 128) | wjCus0 (custom) |
|-----------------|---|---------------------------|-----------------|
| `riscv32i` / `rv32i` | | | |
| `riscv32im` / `rv32im` | ✓ | | |
| `rv32i_zve32x_zvl128b` | | ✓ | |
| `rv32im_zve32x_zvl128b` | ✓ | ✓ | |
| `riscv32i_wjCus0` / `riscv32im_wjCus0` | (im has ✓) | | ✓ |

The `wjCus0` variants enable a custom coprocessor extension used by the MNIST app. Full matrix details and target-string handling live in [`remu_hal/README-targets.md`](remu_hal/README-targets.md).

---

## Repository layout (overview)

| Area | Role |
|------|------|
| `remu_cli` / `remu_debugger` | Interactive shell and debugging commands |
| `remu_simulator` | Simulator abstraction and concrete backends: `simulators/remu`, `simulators/spike`, `simulators/nzea` |
| `remu_state`, `remu_types`, `remu_isa` | Architectural state, buses/devices, ISA typing |
| `remu_hal`, `remu_app/*` | Embedded HAL (`riscv-rt`, `embedded-hal`, `embedded-io`, …) and runnable `no_std` apps: `hello_world`, `collection`, `display`/`shader`, `microbench`, `mnist`, `nes`, `slint` — **[remu_hal/README.md](remu_hal/README.md)** · [中文](remu_hal/README_zh.md) |

The `remu_state` bus models memory regions and devices (UART 16550, SiFive test finisher, CLINT, plus interactive `display`/`mouse`/`keyboard` backed by a winit window). Device/memory configs can come from files via `--dev-base` / `--mem-base`.

---

## Environment & workflow (Nix, direnv, just)

The **supported developer environment is Nix-managed** via [`flake.nix`](flake.nix): **Rust nightly** (with `rust-src`, `clippy`, `rust-analyzer`, `llvm-tools-preview`), **RISC-V bare-metal targets** (`riscv32i` / `im` / `imac` `unknown-none-elf`), **Verilator**, **clang/libclang**, **mold**, **qemu**, and **`just`**.

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

**`run-app` platform selection:** the `--platform` recipe argument routes to different runners:

| `--platform` | Behavior |
|--------------|----------|
| `remu` (default) | Run under `remu_cli` with the built-in simulator |
| `spike` | Build a Spike-appropriate ELF and run it under the native `spike` binary with `--real-time-clint` |
| `qemu` | Run under `qemu-system-riscv32` |
| `host` | Run the app natively on the host (Rust `std`) — no simulator |

**Temporary env vars (`run-app` / embedded `cargo run`):** the app runner (`remu_hal/scripts/remu-cargo-runner.sh`) asks **xtask** to print a `remu_cli` command. Set options for that invocation by exporting variables **on the same line** as `just` (or in your shell) so they are visible when the runner runs:

| Variable | Effect |
|----------|--------|
| **`REMU_APP_ARGS`** | Passed to the embedded app via the app-args bridge (`--app-args`); e.g. `REMU_APP_ARGS=ref` for microbench |
| **`DIFFTEST`** | Enable difftest with reference model: `spike` or `remu` (omit / unset = **off**) |
| **`DEV`** | If set (any value), `print run-remu` uses **debug** `remu_cli` (`cargo run -p remu_cli` without `--release`). **Embedded `remu_app_*` stays `--release`** (`run-app` / `build-app` unchanged) |
| **`BATCH`** | If set (any value), adds `--batch --startup continue` for non-interactive runs |

Example: run **microbench** on **remu** at `ref` scale, non-interactively:

```bash
REMU_APP_ARGS=ref BATCH=true just run-app microbench riscv32im
```

Example: run **mnist** with **Spike** as difftest reference:

```bash
DIFFTEST=spike just run-app mnist riscv32im_zve32x_zvl128b
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

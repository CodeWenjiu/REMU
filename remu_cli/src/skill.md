---
name: remu
description: Use this skill when you need to invoke remu for difftest debugging, run embedded apps, or understand remu CLI usage. Covers batch mode, watchdog, sim-options, and interactive commands.
---

# remu

remu is a RISC-V simulator and difftest debugger.
Use this skill whenever you need to invoke remu for difftest debugging, run embedded apps, or understand remu CLI usage.

## Part 1 — remu Project

### AI Agent Rules (MUST FOLLOW)

When working with remu as an AI agent, **always** apply these rules:

1. **Always use `--batch`** — without it, remu enters an interactive REPL and
   will block forever waiting for stdin. Never invoke remu without `--batch`.

2. **Always enable the watchdog** — append `--sim-opt watchdog=5` when
   using nzea backend. Without it, a deadlocked program will hang indefinitely.
   Format: `MxN` where M = consecutive misses (default 3), N = check interval in seconds.
   Use `watchdog=10` for programs expected to run long between I/O.

3. **Check the exit code** — `GOOD EXIT` means success, `BAD EXIT` means the
   program terminated abnormally. Any other output on stderr is an error.

4. **`--batch` runs `--startup` then `quit`s, no implicit `continue`** — the
   startup script must end with `continue` or `step N` to drive program
   execution; otherwise the program simply won't run (no error).

5. **Always enable difftest for RTL verification** — append `--difftest remu`
   when `--platform nzea` is used. The DUT (nzea RTL) is only verified against
   remu's reference model via difftest; without it, a silently wrong DUT can
   produce `GOOD EXIT` and mask real bugs. Omit only when: (a) probing
   non-deterministic behavior impossible to compare, or (b) performance
   benchmarking where diff overhead matters.

### Platforms

remu supports multiple execution platforms for embedded apps:

| Platform | Description |
|----------|-------------|
| `remu` | Built-in RISC-V simulator (default) |
| `qemu` | QEMU system emulation (riscv32 virt machine) |
| `spike` | Spike RISC-V ISA simulator |
| `host` | Native x86_64 (run the app directly on host CPU) |

### just Recipes

#### `run-app APP [target]`

Build and run an embedded app on any platform.

```sh
# Basic usage:
just run-app hello_world riscv32im

# Quick test (debug build, host native):
just run-app microbench riscv32im --platform host --app-args ref

# Full difftest (release build, remu backend):
just run-app microbench riscv32im --platform remu --app-args ref -- --batch --startup '{' continue '}' --difftest remu

# QEMU:
just run-app microbench riscv32im --platform qemu

# Spike:
just run-app microbench riscv32im --platform spike
```

| Arg | Default | Description |
|-----|---------|-------------|
| `APP` | required | App name: hello_world, collection, mnist, microbench |
| `target` | `riscv32i` | ISA target: riscv32i, riscv32im, riscv32imac |
| `--platform` | `remu` | Runtime: remu, qemu, spike, host |
| `--dev` | false | Debug build (faster compile, slower run) |
| `--app-args ARGS` | none | Arguments passed to the embedded app |
| `-- …` | — | All remaining args forwarded to `remu_cli` after `--` |

#### `build-app APP [target]`

Build an embedded app without running it.

```sh
just build-app hello_world riscv32im
just build-app microbench riscv32im --platform spike
```

#### `clean-app`

Remove all app build artifacts (`target/app/`).

```sh
just clean-app
```

***

## Part 2 — remu_cli

remu_cli is the command-line interface to the remu simulator/debugger.

### Quick Invocation

```sh
# Via just (recommended):
just run-app hello_world riscv32im -- --batch --startup '{' continue '}' --difftest remu

# Via cargo directly:
cargo run -p remu_cli --release -- \
  --elf path/to/app.elf --isa riscv32im \
  --platform nzea --difftest remu \
  --batch --startup '{' continue '}' \
  --sim-opt watchdog=5
```

### CLI Options

| Option | Default | Description |
|--------|---------|-------------|
| `--elf PATH` | required | ELF file to load (alias: `--bin`) |
| `--isa SPEC` | `riscv32i` | ISA spec: riscv32i, riscv32im, etc. |
| `--platform PLATFORM` | `remu` | Simulator backend: remu, spike, nzea |
| `--difftest REF` | none | Difftest reference: remu, spike |
| `--batch` | false | Batch mode: run `--startup` then `quit`; no implicit `continue` |
| `--startup TOKENS...` | empty | Commands to run at startup |
| `--sim-opt KEY=VALUE` | none | Backend-specific options |
| `--app-args ARGS` | none | Arguments written to 0x87FF_F000 for the embedded app |
| `--skill` | false | Print this document and exit |

### Backend Sim-Options (`--sim-opt`)

Pass KEY=VALUE pairs. Scoping is determined by `--platform`. Repeat `--sim-opt`
or pass multiple pairs space-separated.

#### nzea

| Key | Values | Example |
|-----|--------|---------|
| `target` | `core`, `tile` (default: `tile`) | `--sim-opt target=tile` |
| `watchdog` | `5`, `3x10`, `3x0.5`, `test`, `off` | `--sim-opt watchdog=3x10` |

```sh
--sim-opt target=core watchdog=5            # multiple pairs in one --sim-opt
--sim-opt target=core --sim-opt watchdog=5  # or repeat --sim-opt
```

### Automation

```sh
# Run to completion (drive program with explicit `continue`):
remu_cli --elf app.elf --isa riscv32im --platform nzea --batch \
  --startup '{' continue '}' --sim-opt watchdog=5

# Step N then stop and dump register before quit:
remu_cli --elf app.elf --isa riscv32im --platform nzea --batch \
  --startup '{' step 10000 '}' and '{' state reg gpr read x10 '}' --sim-opt watchdog=5

# Warm up, enable waveform, run to completion, print statistics:
remu_cli --elf app.elf --isa riscv32im --platform nzea --batch \
  --startup '{' step 10000 '}' and '{' func trace wave-form on '}' and '{' continue '}' and '{' stat print '}' --sim-opt watchdog=5

# Watchdog with custom misses × interval (2 × 0.5s = 1s timeout):
remu_cli --elf app.elf --isa riscv32im --platform nzea --batch \
  --startup '{' continue '}' --sim-opt watchdog=2x0.5
```

### Interactive Commands

| Command | Description |
|---------|-------------|
| `continue` | Run until exit/interrupt |
| `step [N]` | Step N instructions (default 1) |
| `state reg gpr read X` | Read GPR register X |
| `state reg pc write ADDR` | Set PC |
| `state reg csr ...` | Read/write CSRs |
| `state bus ...` | Memory read/write operations |
| `func trace instruction on/off` | Toggle instruction tracing |
| `func trace wave-form on/off` | Toggle waveform tracing |
| `stat print` | Print all statistics |
| `breakpoint set ADDR` | Set breakpoint |
| `breakpoint del ADDR` | Delete breakpoint |
| `quit` | Exit |

Commands can be chained with `and` / `or` (**required** between blocks — adjacent `{ ... } { ... }` without an operator is a parse error). Wrap each command in `{ }` to avoid ambiguity:

```
{ step 10 } and { state reg gpr read x5 }
{ state reg pc write 0x80000000 } and { continue }
```

### Exit Codes

- `GOOD EXIT`: program terminated via sifive_test_finisher with code 0x5555
- `BAD EXIT`: program terminated with other finisher code
- Error output: difftest mismatches, state access errors, deadlock detection
- exit code 0: success; non-zero: failure (check stderr for details)

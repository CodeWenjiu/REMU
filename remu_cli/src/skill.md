# remu Skill

remu is a RISC-V simulator and difftest debugger.
Version: {{version}}

## AI Agent Usage (MUST FOLLOW)

When invoking remu as an AI agent, **always** apply these rules:

1. **Always use `--batch`** — without it, remu enters an interactive REPL and
   will block forever waiting for stdin. Never invoke remu without `--batch`.

2. **Always enable the watchdog** — append `--sim-opt watchdog=5` when
   using nzea backend. Without it, a deadlocked program will hang indefinitely.
   Use `watchdog=10` for programs expected to run long between I/O.

3. **Check the exit code** — `GOOD EXIT` means success, `BAD EXIT` means the
   program terminated abnormally. Any other output on stderr is an error.

4. **Use `--startup` for setup** — any commands that must run before the
   program starts (e.g. setting PC, writing memory) go in `--startup`.

### Quick Invocation Template

```sh
# Via just (recommended):
just run-app hello_world riscv32im --dev -- --batch --difftest remu

# Via cargo directly:
cargo run -p remu_cli --release -- \
  --elf path/to/app.elf --isa riscv32im \
  --platform nzea --difftest remu \
  --batch \
  --sim-opt watchdog=5
```

### Recognizing Results

- stdout: `GOOD EXIT` or `BAD EXIT`
- stderr: difftest mismatches, state access errors, deadlock detection
- exit code 0: success; non-zero: failure (check stderr for details)

## Quick Start

```sh
# Build and run an embedded app (hello_world on riscv32im):
just run-app hello_world riscv32im

# With difftest reference:
just run-app hello_world riscv32im -- --difftest remu

# In debug mode (faster builds):
just run-app hello_world riscv32im --dev

# With nzea backend + deadlock watchdog:
just run-app hello_world riscv32im -- --platform nzea --sim-opt watchdog=5
```

## CLI Invocation

```sh
# Run an ELF directly:
cargo run -p remu_cli --release -- --elf path/to/app.elf --isa riscv32im

# With platform and difftest:
cargo run -p remu_cli --release -- --elf app.elf --isa riscv32im \
  --platform nzea --difftest remu
```

## CLI Options

| Option | Default | Description |
|--------|---------|-------------|
| `--elf PATH` | required | ELF file to load |
| `--isa SPEC` | `riscv32i` | ISA spec: riscv32i, riscv32im, etc. |
| `--platform PLATFORM` | `remu` | Simulator backend: remu, spike, nzea |
| `--difftest REF` | none | Difftest reference: remu, spike |
| `--batch` | false | Batch mode: auto-continue, then quit |
| `--startup TOKENS...` | empty | Commands to run at startup |
| `--sim-opt KEY=VALUE` | none | Backend-specific options |
| `--skill` | false | Print this document and exit |

## Backend Sim-Options (`--sim-opt`)

Pass KEY=VALUE pairs. Scoping is determined by `--platform`. Repeat `--sim-opt`
or pass multiple pairs space-separated.

### nzea

| Key | Values | Example |
|-----|--------|---------|
| `target` | `core`, `tile` (default: `tile`) | `--sim-opt target=tile` |
| `watchdog` | `5`, `3x10`, `test`, `off` | `--sim-opt watchdog=3x10` |

```sh
--sim-opt target=core watchdog=5           # multiple pairs in one --sim-opt
--sim-opt target=core --sim-opt watchdog=5  # or repeat --sim-opt
```

## Automation

```sh
# Batch mode: run startup commands, continue, then quit
remu_cli --elf app.elf --isa riscv32im --batch --startup '{' state reg pc write 0x80000000 '}' -- --platform nzea

# Watchdog detects deadlock (PC unchanged for N x interval):
remu_cli --elf app.elf --isa riscv32im --batch --sim-opt watchdog=5
```

## Interactive Commands

| Command | Description |
|---------|-------------|
| `continue` | Run until exit/interrupt |
| `step [N]` | Step N instructions (default 1) |
| `state reg gpr read X` | Read GPR register X |
| `state reg pc write ADDR` | Set PC |
| `state reg csr ...` | Read/write CSRs |
| `state bus ...` | Memory read/write operations |
| `func trace instruction on/off` | Toggle instruction tracing |
| `func trace waveform on/off` | Toggle waveform tracing |
| `stat` | Print statistics |
| `breakpoint set ADDR` | Set breakpoint |
| `breakpoint del ADDR` | Delete breakpoint |
| `quit` | Exit |

Commands can be chained with `and` / `or`. Wrap each command in `{ }` to avoid ambiguity:

```
{ step 10 } and { state reg gpr read x5 }
{ state reg pc write 0x80000000 } and { continue }
```

## Exit Codes

- `GOOD EXIT`: program terminated via sifive_test_finisher with code 0x5555
- `BAD EXIT`: program terminated with other finisher code
- Error output: difftest mismatches, state access errors, etc.

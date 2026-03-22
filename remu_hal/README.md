# remu_hal

**English** | [简体中文](README_zh.md)

**Embedded HAL** for **RISC-V bare-metal** (`no_std`) programs running on remu: **`riscv-rt`**, **`embedded-hal` / `embedded-io`**, UART, heap, traps, clean exit.

**Targets, Zve, xtask:** [README-targets.md](README-targets.md)  
**Repository overview:** [README.md](../README.md) · [README_zh.md](../README_zh.md)

---

## 1. Goals

Expose remu’s **MMIO devices** (UART, test finisher, CLINT, …) as a **reusable HAL** aligned with the **embedded Rust** stack:

- **`riscv-rt`** — entry, `memory.x` + `link.x`, trap vector contract.
- **`embedded-hal` (1.x)** — common trait boundary for future GPIO, delays, third-party drivers.
- **`embedded-io`** — byte-stream traits; **16550 UART** implements **`embedded_io::Write`** today.
- **`riscv`** + **`critical-section`** (`critical-section-single-hart`) — CSRs and single-hart critical sections.
- **`embedded-alloc` (LlffHeap)** + linker heap symbols — **`Vec` / `String` / `Box`**.
- **`panic-halt`** — default panic; unhandled M-mode traps print **`mcause` / `mepc` / `mtval`** on UART then panic.

Apps **only depend on `remu_hal`** for `#[entry]`, `println!`, UART, exit, heap setup—no long list of embedded crates in each app.

---

## 2. Module overview

| Area | Role |
|------|------|
| **`riscv-rt`** | Re-exports **`entry`**; `build.rs` publishes **`memory.x`** for the linker with **`riscv-rt`**’s **`link.x`**. |
| **`cpu::pre_main_init`** | Pre-`main` CPU setup (call first in `main`, or via **`heap::init`**). |
| **`uart::Uart16550`** | MMIO 16550-style TX; **`embedded_io::Write`** + **`core::fmt::Write`**; default base in **`addresses`**. |
| **`print` / `println!`** | Formatted output on the default UART. |
| **`time`** | Read CLINT **`mtime`** (10 MHz convention in remu—see source). |
| **`exit_success` / `exit_failure`** | SiFive-style test finisher MMIO → remu good/bad exit. |
| **`heap::init`** | **`#[global_allocator]`**; call **`unsafe { init() }` once** before any heap use (includes **`pre_main_init`**). |
| **`trap`** | **`ExceptionHandler` / `DefaultHandler`** — UART diagnostics, then panic. |

Triple / **Zve** / one-shot **xtask** commands: **[README-targets.md](README-targets.md)**.

---

## 3. Example apps (`remu_app/*`)

Independent **`no_std`** crates with **`riscv-rt` `#[entry]`**, linked against remu’s machine; runnable under **remu CLI** with workspace **xtask** / **`-Z build-std=core,alloc`**. **`mnist`** is **handwritten-digit (MNIST-style) classification**, not a generic “ML demo”.

| Crate | Path | Purpose |
|-------|------|---------|
| **`remu_app_hello_world`** | `remu_app/hello_world` | Minimal UART **Hello World**, **`exit_success`** — boot + UART + exit. |
| **`remu_app_collection`** | `remu_app/collection` | After **`heap::init`**, exercises **`Vec` / `String` / `Box`**. |
| **`remu_app_mnist`** | `remu_app/mnist` | **MNIST digit recognition**: quantized FC net (784→256→128→10), **`test_images/`**, optional **`BENCHMARK_MODE`**. |

Build/run (RV32IM, Zve32x, …): **[README-targets.md](README-targets.md)** — **`xtask print run-app`**, **`just run-app`**, or manual **`cargo`**. Use **`-Z build-std=core,alloc`** for apps.

---

## 4. Dependency sketch

```
remu_app_*  ──depends on──►  remu_hal
                               │
       ┌───────────────────────┼───────────────────────┐
       ▼                       ▼                       ▼
  riscv-rt              embedded-io              embedded-hal
  riscv                 embedded-alloc           (traits / future drivers)
  critical-section      panic-halt
```

---

## 5. Summary

- **`remu_hal`** stacks **`riscv-rt` + `embedded-hal` / `embedded-io` + `riscv` + `embedded-alloc`** for a **portable embedded HAL** on remu’s simulated hardware.
- **`remu_app`** holds **several runnable tests/samples**: minimal I/O, **heap + collections**, **MNIST inference**.

New MMIO devices: prefer **`embedded-hal` / `embedded-io`** traits and extend the table here.

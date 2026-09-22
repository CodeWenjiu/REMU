# rcore — an operating system on remu

rCore (per the [rCore-Tutorial-Book v3](https://rcore-os.cn/rCore-Tutorial-Book-v3/)) built
on the remu stack. This directory is **a separate product line** from `remu_app/*`: the
standalone apps there are M-mode programs wired straight to MMIO; rcore is an S-mode
kernel that serves U-mode user programs.

## Layers

```
U mode   user programs        ── syscall (ecall, traps into S mode)       ─┐
S mode   kernel               ── SBI     (ecall, traps into M mode)       ─┤ this directory
M mode   firmware             (remu_firmware / QEMU: OpenSBI)             ─┘
```

Three interfaces, three owners — do not blur them:

| Interface | Between | Consumer | Owner |
|---|---|---|---|
| MMIO | M-mode program ↔ hardware | standalone apps | `remu_hal_embedded` |
| **SBI** | S-mode kernel ↔ M-mode firmware | **kernel** | **kernel's own `sbi.rs`** |
| syscall | U-mode program ↔ S-mode kernel | user programs | `remu_hal_rcore` (`remu_hal/rcore/`) |

SBI belongs to the kernel, not to any HAL crate: the firmware serves it —
`remu_firmware` (loaded with `--firmware`) on remu, OpenSBI on QEMU — and the
kernel wraps the two legacy extension IDs it needs. No shared crate: ~30 lines
with a single consumer does not justify one, and the IDs are spec constants
that both sides hardcode (same convention the project uses for MMIO register
maps).

## Layout

| Path | Role |
|---|---|
| `kernel/` | The kernel (`rcore_kernel`) — S mode |
| `firmware/` | The firmware (`remu_firmware`) — M mode |
| `rcore64.json` | Custom rustc target (`target_os = "rcore"`) for user programs |

## Chapter 2 status (batch system)

`kernel/` boots on remu, enters a user program in U mode, and serves its
`ecall`s:

- **kernel** (S mode): own linker script pinned at `0x8020_0000`, `_start` in
  `.text.entry`, no `riscv-rt` (the kernel owns its runtime; its build script
  publishes `link.x`). `rust_main` installs `stvec` → `_trap_entry` (assembly:
  swap `sp`/`sscratch`, save 32 GPRs + sepc/scause/stval on the kernel stack,
  dispatch to `trap_handler`, restore, `sret`), then `enter_user()` reads the
  app entry from the boot info and `sret`s into U mode. Syscalls served:
  `sys_write` (64, fd 1 → SBI `console_putchar`) and `sys_exit` (93 → SBI
  `shutdown`); unknown ids return `-ENOSYS`.
- **firmware** (M mode): installs `mtvec`, validates the boot info handed over
  in `a1`, serves the kernel's SBI `ecall`s, hands over with `mret`. On QEMU
  this role is OpenSBI; the kernel image is identical across platforms.
- **user programs** (`remu_hal_rcore`, the rcore arm of `remu_hal`):
  `_start` in `.text.entry` (clear `.bss`, call `main`, `sys_exit` the return
  value), `Uart16550` = fd-1 `sys_write` writer, exit helpers = `sys_exit`,
  plus a panic handler that reports and exits. Apps stay platform-agnostic:
  `#[remu_hal::entry]` + the HAL API compile unchanged from bare-metal/
  host. Built with the custom `rcore64.json` target (`os = "rcore"`) so no
  bare-metal M-mode runtime (riscv-rt / remu_hal_embedded) is dragged in;
  `remu_hal_rcore`'s build script publishes the user `link.x` (base
  `0x8040_0000`).

### Building & running

`rcore` is a **platform** in the xtask/`run-app` vocabulary: the firmware and
kernel are injected automatically, and the app is an ordinary `remu_app_*`
package built as a U-mode program:

```sh
just build-app hello_world riscv64im --platform rcore   # build firmware + kernel + user app
just run-app hello_world riscv64im --platform rcore     # interactive REPL (continue / step / inspect)
# Non-interactive run once:
just run-app hello_world riscv64im --platform rcore -- --batch --startup continue
```

Only the remu backend is supported for now, and the target must be
`riscv64im`. The app runs in U mode from `0x8040_0000` (rCore convention),
under the kernel at `0x8020_0000` and the firmware at `0x8000_0000`.

Expected output:

```
rcore kernel: chapter 2
Hello World
Answer: 42
Quiting...
```

### Firmware (`--firmware`): a real M-mode firmware

`--firmware <PATH>` loads an extra ELF image **besides** `--elf`. It is
optional; when given, the firmware's ELF entry becomes the reset PC, so no
address needs to be configured — the firmware runs first and hands over to the
program itself.

```sh
remu_cli --firmware remu_firmware --elf kernel.elf --app hello.elf --isa riscv64im --platform remu ...
```

All images load at their own link addresses (firmware at the reset region
`0x8000_0000`, kernel at `0x8020_0000`, app at `0x8040_0000`). `--app` is
optional; its ELF entry is published in the boot info (`app_entry`) for the
kernel to run.

On remu the firmware is a **real M-mode program** (`remu_firmware`, this
directory's `firmware/`): it installs its own `mtvec`, hands over to the
S-mode kernel with `mret`, and serves the kernel's `ecall` (SBI) from its trap
vector — the same role OpenSBI plays on QEMU. remu contributes privilege
levels + trap semantics (`ecall` traps by mode: U→S via `stvec`, S→M via
`mtvec`; `sret`/`mret` restore the saved mode); the firmware contributes the
policy.

### Boot convention (a0/a1) and the boot info block

Per the RISC-V boot convention the firmware starts with `a0` = hartid and
`a1` = a pointer to a boot-info block (`remu_state::bus::BootInfo`): the
resolved device map (UART / finisher / CLINT bases) plus the kernel entry
point and the user-program entry point. remu writes it at `0x87FF_E000` when
a firmware is loaded. The firmware reads the device addresses from there —
nothing is hardcoded, so the device map stays user configuration
(`--dev-base` / `--dev-addon` / `--dev`). The layout is mirrored on the
firmware side (`firmware/src/boot_info.rs`); it is a fixed ABI, not a shared
crate (the firmware is zero-dependency).

### What each simulator does (for comparison)

| | Reset entry | "Firmware" content | I/O service path |
|---|---|---|---|
| **Spike** | `0x1000` (`DEFAULT_RSTVEC`) | An 8-instruction ROM trampoline (reads the DTB pointer, `mhartid`, jumps to the ELF entry) + DTB | **HTIF** (`tohost`/`fromhost` mailboxes + host fesvr thread); Spike itself has **no SBI** |
| **QEMU virt** | `0x1000` | `-bios <image>` — a real firmware (OpenSBI by default); `-bios none` jumps straight to `-kernel` | **SBI** (OpenSBI) or direct devices |
| **remu** | `--init-pc`, or the `--firmware` entry when given | Optional real firmware image; otherwise none | Firmware's `mtvec` trap handler (M mode) |

So Spike and QEMU answer the same question differently: Spike **bypasses SBI**
with host-assisted HTIF, QEMU **loads a real firmware**. `--firmware` follows
the QEMU model, which is the one the rcore line needs. On remu the
simulator's only role is delivering the trap; the firmware image itself
decides what `ecall` means.

## Why no `gc`/`imac`

The kernel target is `riscv64im`: remu does not implement the C (compressed) or A (atomic)
extensions, so a compiler emitting `c.*` or `amoadd.*` would fault. `riscv64im` is also
what the difftest reference accepts.

## Next chapters

- **ch3** (multiprogramming / task switching): multiple user programs loaded
  side by side, a task manager, and timer-driven preemption (CLINT timer +
  `stimecmp`/S-mode interrupt delivery). The kernel currently serves exactly
  one program and has no scheduler.
- SBI does **not** go into `remu_hal_embedded`: those apps are M-mode programs that reach
  the hardware directly, so an SBI hop would be both backwards and a lie.
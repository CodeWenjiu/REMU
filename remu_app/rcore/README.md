# rcore — an operating system on remu

rCore (per the [rCore-Tutorial-Book v3](https://rcore-os.cn/rCore-Tutorial-Book-v3/)) built
on the remu stack. This directory is **a separate product line** from `remu_app/*`: the
standalone apps there are M-mode programs wired straight to MMIO; rcore is an S-mode
kernel that serves user programs.

## Layers

```
U mode   user programs        ── syscall (ecall, S mode traps in)     ─┐
S mode   kernel               ── SBI     (ecall, M mode traps in)     ─┤ this directory
M mode   firmware             (remu_firmware / QEMU: OpenSBI)         ─┘
```

Three interfaces, three owners — do not blur them:

| Interface | Between | Consumer | Owner |
|---|---|---|---|
| MMIO | M-mode program ↔ hardware | standalone apps | `remu_hal_embedded` |
| **SBI** | S-mode kernel ↔ M-mode firmware | **kernel** | **kernel's own `sbi.rs`** |
| syscall | U-mode program ↔ S-mode kernel | user programs | `remu_hal_rcore` (future, ch2) |

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

## Chapter 1 status

`kernel/` boots on remu (and mirrors the QEMU boot flow):

- own linker script pinned at `0x8020_0000` (QEMU `virt` convention — OpenSBI occupies
  `[0x8000_0000, 0x8020_0000)`; on remu the firmware sits there instead)
- `_start` in `.text.entry` (first instruction sets `sp`, then calls `rust_main`)
- `console_putchar` (EID 1) and `shutdown` (EID 8) over SBI
- no `riscv-rt`: the kernel owns its runtime (entry/stack/layout); the build script
  publishes its `link.x` into `OUT_DIR` so the workspace-wide `-Tlink.x` resolves to the
  kernel layout

`firmware/` is the M-mode half (the same role OpenSBI has on QEMU): it installs
`mtvec`, validates the boot info handed over in `a1`, hands control to the
S-mode kernel with `mret`, and serves the kernel's `ecall` from its trap
vector. Both are zero-dependency binaries on `riscv64im-unknown-none-elf`.

Run on remu:

```sh
just build-os
just run-os             # builds firmware + kernel, runs with --firmware + --elf
```

Expected output:

```
rcore kernel: chapter 1
hello from S mode via SBI
```

### Firmware (`--firmware`): a real M-mode firmware

`--firmware <PATH>` loads an extra ELF image **besides** `--elf`. It is
optional; when given, the firmware's ELF entry becomes the reset PC, so no
address needs to be configured — the firmware runs first and hands over to the
program itself.

```sh
remu_cli --firmware remu_firmware --elf kernel.elf --isa riscv64im --platform remu ...
```

Both images load at their own link addresses (firmware at the reset region
`0x8000_0000`, the program at e.g. `0x8020_0000`).

On remu the firmware is a **real M-mode program** (`remu_firmware`, this
directory's `firmware/`): it installs its own `mtvec`, hands over to the
S-mode kernel with `mret`, and serves the kernel's `ecall` (SBI) from its trap
vector — the same role OpenSBI plays on QEMU. remu contributes privilege
levels + trap semantics (`ecall` traps to `mtvec`, `mret` restores the saved
mode); the firmware contributes the policy.

### Boot convention (a0/a1) and the boot info block

Per the RISC-V boot convention the firmware starts with `a0` = hartid and
`a1` = a pointer to a boot-info block (`remu_state::bus::BootInfo`): the
resolved device map (UART / finisher / CLINT bases) plus the kernel entry
point. remu writes it at `0x87FF_E000` when a firmware is loaded. The
firmware reads the device addresses from there — nothing is hardcoded, so the
device map stays user configuration (`--dev-base` / `--dev-addon` / `--dev`).
The layout is mirrored on the firmware side (`firmware/src/boot_info.rs`); it
is a fixed ABI, not a shared crate (the firmware is zero-dependency).

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

- **ch2** adds the first user program (U mode): the kernel needs `sstatus`/`stvec`/
  `sepc`/`satp` CSRs and S-mode trap delivery in remu, plus the user-side HAL
  (`remu_hal_rcore`) — created when the first user program appears, not before.
- SBI does **not** go into `remu_hal_embedded`: those apps are M-mode programs that reach
  the hardware directly, so an SBI hop would be both backwards and a lie.
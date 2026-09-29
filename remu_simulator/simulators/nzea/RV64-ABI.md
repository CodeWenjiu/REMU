# nzea RV64: interface status (handoff)

Status: **RV64 runs end to end.** The two blockers from nzea's bring-up report (narrow MMIO devices,
silently dropped bus errors) are fixed on the remu side, and both acceptance runs pass. The ABI type
question is settled: remu accepts the signed spelling CIRCT emits (`int` / `longint` / `bit`) — the widths
are explicit, which is what mattered.

One item back to nzea: **re-dumping `riscv32i` currently fails inside nzea's elaboration** (§8.1), which
blocks the RV32-on-RTL regression until it is fixed.

Audience: nzea (RTL/DPI side). §6–§7 are remu-internal context.

---

## 1. Scope

Only the remu ↔ nzea boundary: the DPI declarations (`bus_read` / `bus_write` / `commit_trace`), the ISA
string remu passes to `just dump`, and the value widths that cross the boundary.

## 2. Interface

### 2.1 DPI declarations (measured)

Widths follow `CoreConfig.width` (`= isaConfig.xlen`); `csr_addr` / `wstrb` / `gpr_addr` are `pad(32)`;
`csr_valid` / `is_mmio` are 1-bit.

RV32 (`VTop_core_riscv32i__Dpi.h`):

```c
extern void bus_read(int addr, int* rdata);
extern void bus_write(int addr, int wdata, int wstrb);
extern void commit_trace(int next_pc, svBit csr_valid, int csr_addr, int csr_data,
                         int gpr_addr, int gpr_data, svBit is_mmio);
```

RV64 (nzea's dump, SV side):

```verilog
import "DPI-C" context function void bus_read(input longint addr, output longint rdata);
import "DPI-C" context function void bus_write(input longint addr, wdata, input int wstrb);
import "DPI-C" context function void commit_trace(input longint next_pc, input bit csr_valid,
    input int csr_addr, input longint csr_data, input int gpr_addr, input longint gpr_data,
    input bit is_mmio);
```

Widths match this document's expectations field by field; the spelling is CIRCT's signed `int` / `longint`
plus `bit` for the flags, which is exactly what remu's bridge now declares.

### 2.2 How the boundary is wired (remu side)

The C bridge in `nzea_wrapper.cpp` (compiled per model with `-DNZEA_XLEN=32|64`) **defines** the
RTL-facing `bus_read` / `bus_write` / `commit_trace` with the widths above, and forwards **same-width** to
Rust entry points exported through `--dynamic-list=nzea_symbols.list`:

```
bus_read      -> nzea_dpi_read32 / nzea_dpi_read64
bus_write     -> nzea_dpi_write32 / nzea_dpi_write64
commit_trace  -> nzea_dpi_commit32 / nzea_dpi_commit64
```

On the Rust side each width has its own typed channel (`NzeaDpi<u32>` / `NzeaDpi<u64>`, `CommitMsg<u32>` /
`CommitMsg<u64>`) selected at compile time from `P::ISA::XLEN`: nothing is widened or narrowed, and there
is no runtime width branch. `csr_valid` / `is_mmio` are `bool` on the Rust side (1 byte, matching `svBit`).

## 3. What the width-typed boundary solved

An RV64 model calls every symbol with 64-bit arguments. Before the bridge, remu exported RV32-shaped
symbols: `bus_read` would have written 4 bytes through an 8-byte pointer (memory corruption) and
`next_pc` / GPR / CSR values would have been truncated before difftest could see them. The width is now a
compile-time property of the model.

## 4. Requests and constraints

**R1 — DPI types: resolved.** remu **accepts the signed spelling** (`int` / `longint`, `bit` for the
flags). The original ask was "explicit, fixed-width, unsigned"; nzea showed the widths are already explicit
and CIRCT emits signed types (`int` / `longint` share one ABI with their unsigned counterparts), so the
unsigned part is dropped rather than asking CIRCT for a mapping change. remu's bridge is spelled exactly
like the generated declarations and restores unsigned semantics internally.

**R2 — `csr_valid` / `is_mmio` stay 1-bit (`bit` / `svBit`).** Confirmed in nzea's dump; both sides use a
one-byte type. Please keep it.

**R3 — address width = XLEN.** Confirmed: RV64 declares `longint addr`. A 32-bit-cap address space would
have been a limitation, not a bug; no action needed.

**R4 — `wstrb` convention.** Little-endian byte lanes, bit *i* enables byte *i*; sub-word stores drive the
low lanes. remu implements the masked paths on exactly this convention, and now folds a wide access onto
narrow devices accordingly (see §6).

**R5 — ISA strings and layout.** `riscv64i` / `riscv64im` are what remu passes to `just dump --isa`; the
layout `<outDir>/<target>/<isa>/` with `NzeaCore.stats.toml` beside `filelist.f` is unchanged.

**R6 — RV64 dump availability: resolved** (the dump exists; §8 runs).

## 5. If nzea prefers a different ABI

| nzea choice | consequence on the remu side |
|---|---|
| Widths follow XLEN, signed spelling (current) | nothing to do — this document's implementation matches |
| Pin to 64-bit everywhere (RV32 zero-extends) | remu deletes the 32-bit bridge branch and its `nzea_dpi_*32` entry points; all RV32 models must be re-dumped/rebuilt |
| Pin to 32-bit everywhere | not viable: truncates PC/GPR in RV64 (R3) |

## 6. Fixed in response to nzea's bring-up report

**Narrow MMIO devices accept XLEN-wide accesses.** `Bus` used to require the whole access range to fit
inside a device window, and devices only implemented their natural width, so an RV64 `sw` to the 4-byte
sifive_test finisher (or the UART) failed with `unmapped`. Now:

- device lookup for wide accesses matches on the access's first byte (`find_device_at`);
- `DeviceAccess::access_width()` (default 32, CLINT = 64) tells the bus how wide a device is;
- a masked 64-bit write is folded per 32-bit group with read-modify-write (strobe semantics preserved), and
  a 64-bit read composes the covered 32-bit groups (bytes outside the window read as zero);
- a device that genuinely has 64-bit registers (CLINT) keeps atomic 64-bit accesses;
- a strobed byte outside the device window is `unmapped`, not silently dropped.

**DPI bus errors are no longer swallowed.** `dpi_read` / `dpi_write` used to keep only
`BusError::ProgramExit`; everything else was dropped, which is why the failing finisher write looked like a
livelock (no output, no error, watchdog silent by design because commits continued). Now any other
`BusError` is recorded and surfaced by `step_once` as a state access error, so the run stops with a
diagnostic (`unmapped`, `UnsupportedAccessWidth`, …).

## 7. remu-side follow-ups (not blocking)

- **CSR width**: remu's CSR model is 32-bit; RV64 CSR values keep their low word. difftest with remu as the
  reference compares PC/GPR/FPR/VR, not CSRs, so nothing blocks. Widening touches the spike protocol too.
- **Breakpoint addresses** are u32; RV64 PCs above 4 GiB cannot be breakpointed yet.
- **`riscv64i` app builds need a target spec**: rustc ships `riscv64im-unknown-none-elf` and
  `riscv64imac-unknown-none-elf` but **not** `riscv64i-unknown-none-elf`. Running a pure-I RV64 acceptance
  therefore needs a small JSON target spec in `remu_app/remu_hal/` plus an xtask mapping; until then §8
  covers `riscv64im` only. Tell remu if a pure-I RV64 configuration is on the nzea roadmap.
- **`misa` of the Zve variants** does not set the V bit (bit 21); worth deciding separately.

## 8. Acceptance — measured

```
just run-app hello_world riscv64im --platform remu -- --platform nzea --difftest remu --batch \
    --startup '{' continue '}' --sim-opt watchdog=3x5
```
→ `Hello World` / `Answer: 42`, exit 0, no difftest mismatch (was: hang in the finisher spin).

```
just run-app microbench riscv64im --platform remu -- --platform nzea --difftest remu --batch \
    --app-args test --startup '{' continue '}' and '{' stat ipc '}' --sim-opt watchdog=3x5
```
→ all benchmarks PASS; `stat_cycle` 2288740, `stat_inst_commit` 536994, `ipc` 0.2346 (RV32 reference:
0.2173 — same range).

### 8.1 Blocking issue on the nzea side (found while re-verifying RV32)

Re-dumping **`riscv32i`** currently fails inside nzea's own elaboration:

```
Exception in thread "main" java.lang.IllegalArgumentException: requirement failed: fill expects n >= 1, got 0
    at nzea_rtl.stdlib.Util$package$.fill(Util.scala:90)
    at nzea_core.back.int.Alu$.architecture(Alu.scala:110)
```

`riscv64im` dumps fine (the runs above), so this is RV32-only. Because the bridge source is part of the model
hash, remu's bridge change invalidates the cached `riscv32i` `.so` — so the RV32 **nzea** regression cannot be
re-run until that dump works again; please fix it (or tell remu it is expected). RV32 was re-verified on the
remu backend meanwhile (`hello_world riscv32i`: `Hello World` / `Answer: 42`, `cargo test --workspace` green).

## 9. Out of scope here

- Instruction-level RV64 semantics and RTL porting (nzea side).
- D/F/V extensions on nzea.
- Multi-region stats, windowed sampling, and other stat-schema v2 topics.

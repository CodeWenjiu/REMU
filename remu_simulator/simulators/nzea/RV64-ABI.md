# nzea RV64: interface changes remu needs (handoff, draft)

Status: **remu side is implemented and verified on RV32; one request is pending** (R1 below: explicit
unsigned fixed-width DPI types). No RV64 model has run yet — the RV64 path is written against the shape
nzea generates today (widths follow `CoreConfig.width`), and R1 would replace the remaining inference with a
declared contract.

Audience: nzea (RTL/DPI side). Everything outside §2–§5 is remu-internal and listed for context only.

---

## 1. Scope

Only the remu ↔ nzea boundary: the DPI declarations (`bus_read` / `bus_write` / `commit_trace`), the ISA
string remu passes to `just dump`, and the value widths that cross the boundary. Instruction semantics and
RTL internals are nzea's; this document does not ask for any RTL behaviour change.

## 2. Current interface

### 2.1 DPI declarations follow the signal width

`IbusDpiBridge` / `DbusDpiBridge` declare `bus_read` / `bus_write` with `bus.req.bits.addr` / `wdata` at
`CoreConfig.width` (`= isaConfig.xlen`), and `CommitDpiBridge` passes `next_pc` / `csr_data` / `rd_value`
the same way. `csr_addr` and `wstrb` are explicitly `pad(32)`; everything else keeps its declared width.

Verilator therefore emits, for the RV32 model
(`target/remu-so/nzea/core/riscv32i/verilator_build/VTop_core_riscv32i__Dpi.h`, measured):

```c
extern void bus_read(int addr, int* rdata);
extern void bus_write(int addr, int wdata, int wstrb);
extern void commit_trace(int next_pc, svBit csr_valid, int csr_addr, int csr_data,
                         int gpr_addr, int gpr_data, svBit is_mmio);
```

Under the same rule an RV64 model would ask for `long long` on every XLEN-wide field: `addr` / `wdata` /
`next_pc` / `csr_data` / `gpr_data`, and `rdata` through a `long long*`. R1 asks nzea to make those types
explicit and unsigned instead — the table there shows the target shape.

### 2.2 How the boundary is wired (remu side, already implemented)

remu no longer exports the RTL-facing symbols. The C bridge in `nzea_wrapper.cpp` (compiled per model with
`-DNZEA_XLEN=32|64`) **defines** `bus_read` / `bus_write` / `commit_trace` with the width that model's
SystemVerilog declares, and forwards **same-width** to Rust entry points exported through
`--dynamic-list=nzea_symbols.list`:

```
bus_read        -> nzea_dpi_read32       (int / int*)        | nzea_dpi_read64       (long long / long long*)
bus_write       -> nzea_dpi_write32      (int/int/int)       | nzea_dpi_write64      (long long/long long/int)
commit_trace    -> nzea_dpi_commit32     (int, svBit, …)     | nzea_dpi_commit64     (long long, svBit, …)
```

On the Rust side each width has its own typed channel (`NzeaDpi<u32>` / `NzeaDpi<u64>`, `CommitMsg<u32>` /
`CommitMsg<u64>`), selected at compile time by the policy's XLEN: **no value is widened or narrowed at the
boundary, and there is no runtime width branch** — an RV32 run never touches a 64-bit value and vice versa.
`is_mmio` / `csr_valid` are `bool` on both sides (1 byte, matching `svBit`), and the bridge already spells the
XLEN-wide fields `unsigned int` / `unsigned long long` (R1's target shape; `int` shares the same ABI).

## 3. The problem this solves

With an RV64 model calling the old RV32-shaped symbols:

| Field | RV32 today | RV64 model will call |
|---|---|---|
| `bus_read.addr` / `rdata` | `int` / `int*` | `long long` / `long long*` |
| `bus_write.addr` / `wdata` | `int` | `long long` |
| `commit_trace.next_pc`, `csr_data`, `gpr_data` | `int` | `long long` |
| `wstrb` (RV64 store) | 4 byte lanes | 8 byte lanes |

A 4-byte write through an 8-byte pointer is memory corruption, not a graceful error; and truncating
`next_pc` / GPR / CSR values before difftest hides real mismatches. The bridge above makes the width a
compile-time property of the model instead.

## 4. What remu needs from nzea

**R1 — generate explicit, unsigned, fixed-width types per XLEN.** Please declare the XLEN-wide fields
with an explicit unsigned integer type instead of leaving the C type to Verilator's signal-width mapping
(the names `bus_read` / `bus_write` / `commit_trace` stay exactly as they are):

| field | RV32 | RV64 |
|---|---|---|
| `addr`, `wdata`, `next_pc`, `csr_data`, `gpr_data` | `int unsigned` → `unsigned int` | `longint unsigned` → `unsigned long long` |
| `csr_addr`, `wstrb`, `gpr_addr` | `int unsigned` | `int unsigned` |
| `csr_valid`, `is_mmio` | `svBit` | `svBit` |

(`bit [31:0]` / `bit [63:0]` are equally fine: Verilator maps them to `IData` / `QData`, which *are*
`uint32_t` / `uint64_t`. What matters is that the width is written down rather than inferred.)

Why:

- it turns the boundary into a contract instead of a guess — today remu can only assume what Verilator will
  emit for XLEN-wide fields on a 64-bit model;
- today's RV32 declaration is **signed** `int`, which forces bit-pattern casts on the remu side
  (`addr as u32 as usize`); unsigned removes that class of subtlety on both sides.

remu's bridge is already written against the unsigned spelling. `int` and `unsigned int` share one ABI, so
models built before this change keep working — no rebuild is required on that account alone.

**R2 — keep `csr_valid` / `is_mmio` as `svBit` (1 bit).** Both sides now use a one-byte type; widening them
to `int` would reintroduce a garbage-high-bits read.

**R3 — address width = XLEN, not "always 32".** If an RV64 model kept 32-bit addresses on the DPI boundary,
remu could not represent PCs or loads above 4 GiB. If the RV64 core is intended to stay inside a 32-bit
physical map, say so explicitly — remu will document it as a limitation instead of a bug.

**R4 — `wstrb` convention at 64 bits**: little-endian byte lanes, bit *i* enables byte *i*; for sub-word
stores the RTL drives the low lanes. remu implements `write_32_masked` / `write_64_masked` on exactly this
convention.

**R5 — confirm the ISA strings and layout.** remu passes `riscv64i` / `riscv64im` to `just dump --isa`
(`NzeaIsa::NZEA_ISA_STR`), and expects the dump directory to stay `<outDir>/<target>/<isa>/` with
`NzeaCore.stats.toml` beside `filelist.f`. `IsaConfig.parse` already accepts `riscv64*`; the request is to
keep it that way (and to say so if the string gains a suffix).

**R6 — tell remu when an RV64 dump exists** (which target first: `core`), so §8 can run.

## 5. If nzea prefers a different ABI

remu's implementation absorbs the width difference on its own side, so nzea needs no change for RV64. If
nzea would rather pin the DPI boundary to one width for all models, that is also fine:

| nzea choice | consequence on the remu side |
|---|---|
| Widths follow XLEN, explicit unsigned types (R1) | nothing to do — the bridge is already written that way; existing `int`-declared models keep working (same ABI) |
| Keep today's implicit `int` / `long long` | still works, but the 64-bit spelling stays an assumption until a real `*__Dpi.h` confirms it — remu would rather not guess |
| Pin to 64-bit everywhere (RV32 zero-extends) | remu deletes the 32-bit branch of the bridge and its `nzea_dpi_*32` entry points; existing RV32 models must be re-dumped/rebuilt |
| Pin to 32-bit everywhere | not viable: truncates PC/GPR in RV64 (R3) |

## 6. What remu has already done (no nzea dependency)

- **ISA wiring**: `NzeaIsaKind::{Rv64I, Rv64Im}` accepted, `NZEA_ISA_STR = "riscv64i" / "riscv64im"`,
  `dispatch_nzea!` arms in `remu_boot`.
- **Width-typed DPI path**: `CommitMsg<W>` and `NzeaDpi<W>` carry the machine word `W`; the per-width
  channels are `NzeaDpi<u32>` / `NzeaDpi<u64>`, installed once per run and selected at compile time from
  `P::ISA::XLEN`. `is_mmio` is `bool`, matching `svBit`.
- **C bridge**: `nzea_wrapper.cpp` defines the RTL-facing symbols with the model's width
  (`-DNZEA_XLEN`), forwarding same-width; `nzea_symbols.list` exports the `nzea_dpi_*` entry points.
- **Bus**: `read_32` / `read_64` and `write_32_masked` / `write_64_masked` (8-lane, little-endian), dispatched
  by type via `BusWord` (`Bus::read_word::<W>` / `write_word_masked::<W>`).
- Verified: RV32 nzea difftest smoke test (`hello_world riscv32i`, `--difftest remu`) passes with the new
  bridge and channels; `cargo test --workspace` green.

## 7. remu-side follow-ups (not blocking RV64 bring-up)

- **CSR width**: remu's CSR model is 32-bit (`Csr` fields, `read_csr`, difftest `diff_mask`, spike's
  `get_csr` protocol). RV64 CSR values keep their low word in commit application and `reg show csr`. This
  does not block: difftest with remu as the reference compares PC/GPR/FPR/VR, not CSRs. Widening the CSR
  storage is a separate work item (it would also touch the spike reference protocol).
- **Breakpoint addresses** are u32 today (`BreakpointHit(u32)`); RV64 PCs above 4 GiB cannot be
  breakpointed until that type widens.
- **`misa` of the Zve variants** does not set the V bit (bit 21); unchanged here, worth deciding separately
  when RV64 difftest compares `misa`.

## 8. Acceptance (once an RV64 model exists)

```
just run-app hello_world riscv64i  -- --platform nzea --difftest remu --batch --startup "{ continue }" --sim-opt watchdog=3x5
just run-app hello_world riscv64im -- --platform nzea --difftest remu --batch --startup "{ continue }" --sim-opt watchdog=3x5
just run-app microbench  riscv64im -- --platform nzea --difftest remu --batch --app-args test \
    --startup "{ continue } and { stat ipc }" --sim-opt watchdog=3x5
```

Expected: `Hello World` / `Answer: 42`, no difftest mismatch, and `stat ipc` in the same range as the RV32
run. `stat print` must list the same schema entries (counter widths up to 64 bits are already supported).

## 9. Out of scope here

- Instruction-level RV64 semantics and RTL porting (nzea side).
- D/F/V extensions on nzea (`fpr`/`VConfig` types exist in the remu table but are unused).
- Multi-region stats, windowed sampling, and other stat-schema v2 topics.

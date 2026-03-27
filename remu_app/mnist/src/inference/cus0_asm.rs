//! Emit CUS0 custom instructions using **GNU `as` / LLVM IAS** `.insn` pseudo-ops.
//!
//! Formats follow the Binutils manual
//! [RISC-V Instruction Formats](https://sourceware.org/binutils/docs/as/RISC_002dV_002dFormats.html).
//! Opcode space **`CUSTOM_0`** is the documented name for custom-0 (`0x0b`).
//!
//! - **NN_LOAD_ACT** — **R-type:** `.insn r CUSTOM_0, 0, 0, x0, {rs1}, {rs2}`  
//! - **NN_START** — **I-type:** `.insn i CUSTOM_0, 1, x0, 0(x0)`.
//! - **NN_LOAD** — **I-type:** `.insn i CUSTOM_0, 2, {rd}, 0({bias})` — **`rd`** = destination,
//!   **`bias`** (`rs1`) = extract index. Wrapped by [`emit_nn_load`]; pipeline calls it once per logit.
//!   Bias is carried in a GPR at execute time, so repeated calls may share the same machine pattern
//!   with different runtime values in `rs1`.

/// **NN_LOAD_ACT**: `rs1` = bias, `rs2` = value.
#[inline(always)]
#[cfg(target_arch = "riscv32")]
fn emit_nn_load_act(rs_bias: i32, rs_val: i32) {
    unsafe {
        core::arch::asm!(
            ".insn r CUSTOM_0, 0, 0, x0, {rs1}, {rs2}",
            rs1 = in(reg) rs_bias,
            rs2 = in(reg) rs_val,
            options(nostack),
        );
    }
}

/// **NN_START** — I-type (`0x100B`).
#[inline(always)]
#[cfg(target_arch = "riscv32")]
fn emit_nn_start() {
    unsafe {
        core::arch::asm!(
            ".insn i CUSTOM_0, 1, x0, 0(x0)",
            options(nostack),
        );
    }
}

/// **NN_LOAD**: `rd` ← scalar selected by **`bias`** in `rs1` (`imm_i` = 0).
#[inline(always)]
#[cfg(target_arch = "riscv32")]
fn emit_nn_load(dst: &mut u32, bias: i32) {
    unsafe {
        core::arch::asm!(
            ".insn i CUSTOM_0, 2, {rd}, 0({bias_reg})",
            rd = out(reg) *dst,
            bias_reg = in(reg) bias,
            options(nostack),
        );
    }
}

/// `NN_LOAD_ACT` × 784, `NN_START`, then **10× [`emit_nn_load`]** (bias `0..9`) into `logits`.
pub(super) fn emit_pipeline(normalized: &[i8], logits: &mut [u32; 10]) {
    assert_eq!(normalized.len(), 784);
    #[cfg(target_arch = "riscv32")]
    {
        for i in 0..784 {
            emit_nn_load_act(i as i32, normalized[i] as i32);
        }
        emit_nn_start();

        for i in 0..10 {
            emit_nn_load(&mut logits[i], i as i32);
        }
    }
    #[cfg(not(target_arch = "riscv32"))]
    {
        let _ = normalized;
        logits.fill(0);
    }
}

//! RISC-V V extension (OP-V opcode 0x57). Decode only when VLENB > 0.
//! VInst is split by funct3: OpCfg (0b111), OpIvv (0b000), OpMvv (0b010), OpIvi (0b011), OpIvx (0b100), OpMvx (0b110).

remu_macro::mod_flat!(decode, execute);
remu_macro::mod_pub!(op_cfg, op_ivv, op_ivi, op_ivx, op_mvv, op_mvx, utils);

/// funct3 = 0b000: OP-IVV (vector-vector)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) enum OpIvvInst {
    Vor_vv,
}

/// funct3 = 0b111: vsetivli, vsetvli
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) enum OpCfgInst {
    Vsetivli,
    /// vsetvli rd, rs1, vtype: AVL from GPR rs1, 11-bit vtypei
    Vsetvli,
}

/// funct3 = 0b010: OP-MVV
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) enum OpMvvInst {
    Vredsum_vs,
    Vid_v,
    Vmv_x_s,
    Vfirst_m,
    Vsext_vf4,
    Vzext_vf4,
    Vmor_mm,
}

/// funct3 = 0b011: OP-IVI
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) enum OpIviInst {
    Vmerge_vim,
    Vmseq_vi,
    Vmsne_vi,
    Vmv1r_v,
    Vrsub_vi,
    Vadd_vi,
    Vslidedown_vi,
    Vsll_vi,
}

/// funct3 = 0b100: OP-IVX
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) enum OpIvxInst {
    Vmerge_vxm,
    Vadd_vx,
    Vmslt_vx,
    Vmseq_vx,
}

/// funct3 = 0b110: OP-MVX (e.g. vmv.s.x)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) enum OpMvxInst {
    Vmv_s_x,
}

/// Top-level V instruction: one variant per funct3.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VInst {
    OpCfg(OpCfgInst),
    OpIvv(OpIvvInst),
    OpMvv(OpMvvInst),
    OpIvi(OpIviInst),
    OpIvx(OpIvxInst),
    OpMvx(OpMvxInst),
}

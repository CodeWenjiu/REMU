//! RISC-V V extension (OP-V opcode 0x57). Decode only when VLENB > 0.

remu_macro::mod_flat!(decode, execute);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) enum VInst {
    Vsetivli,
    /// vsetvli rd, rs1, vtype: AVL from GPR rs1, 11-bit vtypei
    Vsetvli,
    Vid_v,
    Vrsub_vi,
    Vadd_vi,
    Vmerge_vim,
    /// vmseq.vi: vd[i] = (vs2[i] == simm5) ? 1 : 0 (mask, 1 bit per element)
    Vmseq_vi,
    /// vmerge.vxm: vd[i] = v0[i] ? rs1 (scalar) : vs2[i]
    Vmerge_vxm,
    /// vsext.vf4: vd[i] = sign_extend(vs2[i] from SEW/4 to SEW)
    Vsext_vf4,
    /// vmv.s.x: vd[0] = rs1 (scalar), vd[1..] unchanged
    Vmv_s_x,
    /// vmv1r.v: vd = vs2 (copy one vector register, VLEN bytes)
    Vmv1r_v,
    /// vmslt.vx: vd[i] = (vs2[i] < rs1) ? 1 : 0 (signed, mask)
    Vmslt_vx,
    /// vredsum.vs: vd[0] = vs1[0] + sum(vs2[0..vl]), scalar result in vd[0]
    Vredsum_vs,
    /// vmv.x.s: rd = sign_extend(vs2[0]) to XLEN (vm=1 only in Spike)
    Vmv_x_s,
    /// vfirst.m: rd = index of first set mask bit in vs2, or -1 if none (vm=0, same encoding family as vmv.x.s)
    Vfirst_m,
    /// vslidedown.vi: vd[i] = vs2[i+uimm5] if (i+uimm5)<vl else 0
    Vslidedown_vi,
}

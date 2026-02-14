use std::ffi::CString;
use std::marker::PhantomData;
use std::os::raw::c_uint;

use remu_state::bus::{BusOption, MemoryEntry, try_load_elf_into_memory};
use remu_state::{State, StateCmd};
use remu_types::isa::RvIsa;
use remu_types::isa::reg::{Fpr, Gpr, RegAccess};
use remu_types::{AllUsize, DifftestMismatchItem, RegGroup, TracerDyn, Xlen};

use remu_simulator::{
    SimulatorInnerError, SimulatorOption, SimulatorPolicy, SimulatorPolicyOf, SimulatorTrait,
};

use crate::ffi::{
    spike_difftest_copy_mem, spike_difftest_fini, spike_difftest_get_csr, spike_difftest_get_fpr,
    spike_difftest_get_gpr_ptr, spike_difftest_get_pc_ptr, spike_difftest_init,
    spike_difftest_read_mem, spike_difftest_step, spike_difftest_sync_regs_to_spike,
    spike_difftest_sync_mem, spike_difftest_write_mem, DifftestMemLayout, DifftestRegs,
    SpikeDifftestCtx,
};

pub struct SimulatorSpike<P: SimulatorPolicy> {
    ctx: Option<SpikeDifftestCtx>,
    tracer: TracerDyn,
    bus_option: BusOption,
    _marker: PhantomData<P>,
}

impl<P: SimulatorPolicy> SimulatorPolicyOf for SimulatorSpike<P> {
    type Policy = P;
}

impl<P: SimulatorPolicy> SimulatorTrait<P, false> for SimulatorSpike<P> {
    const ENABLE: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        let bus_option = opt.state.bus.clone();

        let mut memory: Vec<MemoryEntry> = bus_option
            .mem
            .iter()
            .map(|region| {
                MemoryEntry::new(region.clone())
                    .expect("invalid memory region spec (should be validated before)")
            })
            .collect();

        try_load_elf_into_memory(&mut memory, &bus_option.elf, &tracer);

        if memory.is_empty() {
            return Self {
                ctx: None,
                tracer,
                bus_option,
                _marker: PhantomData,
            };
        }

        let layout: Vec<DifftestMemLayout> = memory
            .iter()
            .map(|m| DifftestMemLayout {
                guest_base: m.range.start,
                size: m.range.end - m.range.start,
            })
            .collect();

        let init_pc = opt.state.reg.init_pc;
        let init_gpr = [0u32; 32];

        let isa_str = CString::new(P::ISA::ISA_STR).expect("ISA_STR contains null");
        let xlen: c_uint = <<P::ISA as RvIsa>::XLEN as Xlen>::BITS;

        let ctx = unsafe {
            spike_difftest_init(
                layout.as_ptr(),
                layout.len(),
                init_pc,
                init_gpr.as_ptr(),
                xlen,
                isa_str.as_ptr(),
            )
        };

        let ctx = if ctx.is_null() {
            None
        } else {
            let ctx = Some(ctx);
            for m in &memory {
                let (base, ptr, size) = m.difftest_raw_region_read();
                unsafe {
                    spike_difftest_copy_mem(ctx.unwrap(), base, ptr, size);
                }
            }
            ctx
        };

        Self {
            ctx,
            tracer,
            bus_option,
            _marker: PhantomData,
        }
    }

    fn state(&self) -> &State<P> {
        unreachable!("state() must not be called on Spike ref simulator")
    }

    fn state_mut(&mut self) -> &mut State<P> {
        unreachable!("state_mut() must not be called on Spike ref simulator")
    }

    fn step_once<const ITRACE: bool>(&mut self) -> Result<(), SimulatorInnerError> {
        let Some(ctx) = self.ctx else {
            return Err(SimulatorInnerError::RefError(
                "spike difftest not initialized (no memory regions or init failed)".to_string(),
            ));
        };

        let ret = unsafe { spike_difftest_step(ctx) };

        match ret {
            0 => Ok(()),
            1 => Err(SimulatorInnerError::RefError(
                "program exited (ecall exit)".to_string(),
            )),
            _ => Err(SimulatorInnerError::RefError(format!(
                "spike_difftest_step error: {ret}"
            ))),
        }
    }

    fn sync_from(&mut self, dut: &State<P>) {
        let regs = gpr_to_difftest_regs(dut);
        if let Some(ctx) = self.ctx {
            unsafe { spike_difftest_sync_regs_to_spike(ctx, &regs) };
        }

        if let Some(ctx) = self.ctx {
            let raw_regions = dut.bus.mem_regions_for_sync();
            for (base, host_ptr, size) in raw_regions {
                unsafe {
                    spike_difftest_sync_mem(ctx, base, host_ptr, size);
                }
            }
        }
    }

    fn sync_regs_from(&mut self, dut: &State<P>) {
        let regs = gpr_to_difftest_regs(dut);
        if let Some(ctx) = self.ctx {
            unsafe { spike_difftest_sync_regs_to_spike(ctx, &regs) };
        }
    }

    fn regs_diff(&self, dut: &State<P>) -> Vec<DifftestMismatchItem> {
        let Some(ctx) = self.ctx else {
            return vec![];
        };

        let pc_ptr = unsafe { spike_difftest_get_pc_ptr(ctx) };
        let gpr_ptr = unsafe { spike_difftest_get_gpr_ptr(ctx) };
        if pc_ptr.is_null() || gpr_ptr.is_null() {
            return vec![];
        }

        let mut out = Vec::new();
        let ref_pc = unsafe { *pc_ptr };

        if ref_pc != *dut.reg.pc {
            out.push(DifftestMismatchItem {
                group: RegGroup::Pc,
                name: "pc".to_string(),
                ref_val: AllUsize::U32(ref_pc),
                dut_val: AllUsize::U32(*dut.reg.pc),
            });
        }

        for i in 0..32 {
            let r = unsafe { *gpr_ptr.add(2 * i) };
            let d = dut.reg.gpr.raw_read(i);
            if r != d {
                let name = Gpr::from_repr(i)
                    .map(|g| g.to_string())
                    .unwrap_or_else(|| format!("x{i}"));
                out.push(DifftestMismatchItem {
                    group: RegGroup::Gpr,
                    name,
                    ref_val: AllUsize::U32(r),
                    dut_val: AllUsize::U32(d),
                });
            }
        }

        if P::ISA::HAS_F {
            for i in 0..32 {
                let r = unsafe { spike_difftest_get_fpr(ctx, i) };
                let d = dut.reg.fpr.raw_read(i);
                if r != d {
                    let name = Fpr::from_repr(i)
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| format!("f{i}"));
                    out.push(DifftestMismatchItem {
                        group: RegGroup::Fpr,
                        name,
                        ref_val: AllUsize::U32(r),
                        dut_val: AllUsize::U32(d),
                    });
                }
            }
        }

        for slice in P::ISA::csrs_for_difftest() {
            for csr in *slice {
                let mask = csr.diff_mask();
                if mask == 0 {
                    continue;
                }
                let ref_val = unsafe { spike_difftest_get_csr(ctx, csr.addr()) };
                let dut_val = dut.reg.read_csr(*csr);
                if (ref_val & mask) != (dut_val & mask) {
                    out.push(DifftestMismatchItem {
                        group: RegGroup::Csr,
                        name: csr.to_string(),
                        ref_val: AllUsize::U32(ref_val),
                        dut_val: AllUsize::U32(dut_val),
                    });
                }
            }
        }

        out
    }

    fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorInnerError> {
        let Some(ctx) = self.ctx else {
            return Err(SimulatorInnerError::RefError(
                "spike difftest not initialized".to_string(),
            ));
        };

        match subcmd {
            StateCmd::Reg { subcmd } => {
                state_exec_reg(ctx, &self.tracer, subcmd)?;
            }
            StateCmd::Bus { subcmd } => {
                state_exec_bus(ctx, &self.tracer, &self.bus_option, subcmd)?;
            }
        }
        Ok(())
    }
}

impl<P: SimulatorPolicy> Drop for SimulatorSpike<P> {
    fn drop(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            unsafe { spike_difftest_fini(ctx) };
        }
    }
}

fn gpr_to_difftest_regs<P: SimulatorPolicy>(state: &State<P>) -> DifftestRegs {
    let mut gpr = [0u32; 32];
    for i in 0..32 {
        gpr[i] = state.reg.gpr.raw_read(i);
    }
    DifftestRegs {
        pc: *state.reg.pc,
        gpr,
    }
}

fn state_exec_reg(
    ctx: SpikeDifftestCtx,
    tracer: &TracerDyn,
    cmd: &remu_state::reg::RegCmd,
) -> Result<(), SimulatorInnerError> {
    use remu_state::reg::{CsrRegCmd, FprRegCmd, PcRegCmd};

    let pc_ptr = unsafe { spike_difftest_get_pc_ptr(ctx) };
    let gpr_ptr = unsafe { spike_difftest_get_gpr_ptr(ctx) };
    if pc_ptr.is_null() || gpr_ptr.is_null() {
        return Err(SimulatorInnerError::RefError(
            "spike_difftest_get_*_ptr returned null".to_string(),
        ));
    }
    let pc = unsafe { *pc_ptr };

    match cmd {
        remu_state::reg::RegCmd::Pc { subcmd } => match subcmd {
            PcRegCmd::Read => {
                tracer.borrow().reg_show_pc(pc);
            }
            PcRegCmd::Write { value } => {
                let mut new_gpr = [0u32; 32];
                for i in 0..32 {
                    new_gpr[i] = unsafe { *gpr_ptr.add(2 * i) };
                }
                let new_regs = DifftestRegs {
                    pc: *value,
                    gpr: new_gpr,
                };
                unsafe { spike_difftest_sync_regs_to_spike(ctx, &new_regs) };
            }
        },
        remu_state::reg::RegCmd::Gpr { subcmd } => match subcmd {
            remu_state::reg::GprRegCmd::Read { index } => {
                let idx = index.idx();
                let val = unsafe { *gpr_ptr.add(2 * idx) };
                tracer.borrow().reg_show(*index, val);
            }
            remu_state::reg::GprRegCmd::Print { range } => {
                let regs_arr: [(Gpr, u32); 32] = core::array::from_fn(|i| {
                    (Gpr::from_repr(i).expect("valid"), unsafe { *gpr_ptr.add(2 * i) })
                });
                tracer.borrow().reg_print(&regs_arr, range.clone());
            }
            remu_state::reg::GprRegCmd::Write { index, value } => {
                let mut new_gpr = [0u32; 32];
                for i in 0..32 {
                    new_gpr[i] = unsafe { *gpr_ptr.add(2 * i) };
                }
                if index.idx() != 0 {
                    new_gpr[index.idx()] = *value;
                }
                let new_regs = DifftestRegs { pc, gpr: new_gpr };
                unsafe { spike_difftest_sync_regs_to_spike(ctx, &new_regs) };
            }
        },
        remu_state::reg::RegCmd::Fpr { subcmd } => match subcmd {
            FprRegCmd::Read { index } => {
                tracer.borrow().reg_show_fpr(index.idx(), 0); /* Spike difftest has no FPR */
            }
            FprRegCmd::Print { range } => {
                let regs_vec: Vec<(usize, u32)> =
                    (range.start..range.end).map(|i| (i, 0)).collect();
                tracer.borrow().reg_print_fpr(&regs_vec, range.clone());
            }
            FprRegCmd::Write { .. } => { /* Spike difftest has no FPR write; ignore */ }
        },
        remu_state::reg::RegCmd::Csr { subcmd } => match subcmd {
            CsrRegCmd::Read { index } => {
                tracer.borrow().print(&format!("{} = {:#010x}", index, 0)); /* Spike difftest has no CSR */
            }
            CsrRegCmd::Write { .. } => { /* Spike difftest has no CSR write; ignore */ }
        },
    }
    Ok(())
}

fn state_exec_bus(
    ctx: SpikeDifftestCtx,
    tracer: &TracerDyn,
    bus_opt: &BusOption,
    subcmd: &remu_state::bus::BusCmd,
) -> Result<(), SimulatorInnerError> {
    use remu_state::bus::{ReadCommand, WriteCommand};
    use remu_types::DynDiagError;

    match subcmd {
        remu_state::bus::BusCmd::Read { subcmd } => {
            let (addr, width) = match subcmd {
                ReadCommand::U8(a) => (a.addr, 1),
                ReadCommand::U16(a) => (a.addr, 2),
                ReadCommand::U32(a) => (a.addr, 4),
                ReadCommand::U64(a) => (a.addr, 8),
                ReadCommand::U128(a) => (a.addr, 16),
            };
            let mut buf = [0u8; 16];
            let buf_slice = &mut buf[..width];
            let result = if unsafe {
                spike_difftest_read_mem(ctx, addr, buf_slice.as_mut_ptr(), width)
            } == 0
            {
                let v = match width {
                    1 => AllUsize::U8(buf[0]),
                    2 => AllUsize::U16(u16::from_le_bytes([buf[0], buf[1]])),
                    4 => AllUsize::U32(u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]])),
                    8 => AllUsize::U64(u64::from_le_bytes([
                        buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
                    ])),
                    16 => AllUsize::U128(u128::from_le_bytes([
                        buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
                        buf[8], buf[9], buf[10], buf[11], buf[12], buf[13], buf[14], buf[15],
                    ])),
                    _ => unreachable!(),
                };
                Ok(v)
            } else {
                Err(Box::new(remu_state::bus::BusError::unmapped(addr)) as Box<dyn DynDiagError>)
            };
            tracer.borrow().mem_show(addr, result);
        }
        remu_state::bus::BusCmd::Print { addr, count } => {
            const PRINT_BUF_SIZE: usize = 256;
            let count = (*count).min(PRINT_BUF_SIZE);
            let mut buf = [0u8; PRINT_BUF_SIZE];
            let buf_slice = &mut buf[..count];
            let result = if unsafe {
                spike_difftest_read_mem(ctx, *addr, buf_slice.as_mut_ptr(), count)
            } == 0
            {
                Ok(())
            } else {
                Err(Box::new(remu_state::bus::BusError::unmapped(*addr)) as Box<dyn DynDiagError>)
            };
            tracer.borrow().mem_print(*addr, buf_slice, result);
        }
        remu_state::bus::BusCmd::Write { subcmd } => match subcmd {
            WriteCommand::U8 { addr, value } => {
                let bytes = value.to_le_bytes();
                if unsafe { spike_difftest_write_mem(ctx, *addr, bytes.as_ptr(), bytes.len()) } != 0
                {
                    return Err(SimulatorInnerError::RefError(format!(
                        "spike_difftest_write_mem failed: addr={:#x}",
                        addr
                    )));
                }
            }
            WriteCommand::U16 { addr, value } => {
                let bytes = value.to_le_bytes();
                if unsafe { spike_difftest_write_mem(ctx, *addr, bytes.as_ptr(), bytes.len()) } != 0
                {
                    return Err(SimulatorInnerError::RefError(format!(
                        "spike_difftest_write_mem failed: addr={:#x}",
                        addr
                    )));
                }
            }
            WriteCommand::U32 { addr, value } => {
                let bytes = value.to_le_bytes();
                if unsafe { spike_difftest_write_mem(ctx, *addr, bytes.as_ptr(), bytes.len()) } != 0
                {
                    return Err(SimulatorInnerError::RefError(format!(
                        "spike_difftest_write_mem failed: addr={:#x}",
                        addr
                    )));
                }
            }
            WriteCommand::U64 { addr, value } => {
                let bytes = value.to_le_bytes();
                if unsafe { spike_difftest_write_mem(ctx, *addr, bytes.as_ptr(), bytes.len()) } != 0
                {
                    return Err(SimulatorInnerError::RefError(format!(
                        "spike_difftest_write_mem failed: addr={:#x}",
                        addr
                    )));
                }
            }
            WriteCommand::U128 { addr, value } => {
                let bytes = value.to_le_bytes();
                if unsafe { spike_difftest_write_mem(ctx, *addr, bytes.as_ptr(), bytes.len()) } != 0
                {
                    return Err(SimulatorInnerError::RefError(format!(
                        "spike_difftest_write_mem failed: addr={:#x}",
                        addr
                    )));
                }
            }
        },
        remu_state::bus::BusCmd::Set { address, value } => {
            let mut addr = *address;
            for chunk in value.iter() {
                if chunk.is_empty() {
                    continue;
                }
                if unsafe { spike_difftest_write_mem(ctx, addr, chunk.as_ptr(), chunk.len()) } != 0
                {
                    return Err(SimulatorInnerError::RefError(format!(
                        "spike_difftest_write_mem failed: addr={:#x}",
                        addr
                    )));
                }
                addr = addr.saturating_add(chunk.len());
            }
        }
        remu_state::bus::BusCmd::MemMap => {
            let map: Vec<(String, std::ops::Range<usize>)> = bus_opt
                .mem
                .iter()
                .map(|m| (m.name.clone(), m.region.clone()))
                .collect();
            tracer.borrow().mem_show_map(map);
        }
    }
    Ok(())
}

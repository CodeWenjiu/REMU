use remu_state::{State, StateCmd};
use remu_types::isa::reg::RegAccess;
use remu_types::{DifftestMismatchItem, RegGroup, TracerDyn};
use unicorn_engine::{
    Unicorn,
    unicorn_const::{Arch, Mode, Prot, RegisterRISCV},
};

use remu_simulator::{
    from_state_error, SimulatorInnerError, SimulatorOption, SimulatorPolicy, SimulatorPolicyOf,
    SimulatorTrait,
};

const PAGE_SIZE: usize = 4096;

pub struct SimulatorUnicorn<P: SimulatorPolicy> {
    unicorn: Unicorn<'static, ()>,
    state: State<P>,
}

impl<P: SimulatorPolicy> SimulatorPolicyOf for SimulatorUnicorn<P> {
    type Policy = P;
}

impl<P: SimulatorPolicy> SimulatorTrait<P, false> for SimulatorUnicorn<P> {
    const ENABLE: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        let mut state: State<P> = State::new(opt.state.clone(), tracer.clone(), false);
        let mut unicorn = Unicorn::new(Arch::RISCV, Mode::RISCV32)
            .map_err(|e| SimulatorInnerError::RefError(format!("Unicorn init: {}", e)))
            .expect("unicorn new");

        for region in &opt.state.bus.mem {
            let start = region.region.start;
            let len = region.region.end - region.region.start;
            let size = ((len + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
            unicorn
                .mem_map(start as u64, size as u64, Prot::ALL)
                .map_err(|e| SimulatorInnerError::RefError(format!("Unicorn mem_map: {}", e)))
                .expect("mem_map");
            let mut buf = vec![0u8; len];
            if state.bus.read_bytes(start, &mut buf).is_ok() {
                let _ = unicorn.mem_write(start as u64, &buf);
            }
        }

        let pc_u32: <P::ISA as remu_types::isa::RvIsa>::PcState = state.reg.pc;
        let pc = u64::from(*pc_u32);
        unicorn.set_pc(pc).expect("set_pc");
        for i in 0..32u32 {
            let reg = riscv_reg(i);
            let val_u32: <<P::ISA as remu_types::isa::RvIsa>::GprState as RegAccess>::Item =
                state.reg.gpr.raw_read(i as usize);
            let val = u64::from(val_u32);
            unicorn.reg_write(reg, val).expect("reg_write");
        }

        Self { unicorn, state }
    }

    fn state(&self) -> &State<P> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut State<P> {
        &mut self.state
    }

    fn step_once(&mut self) -> Result<(), SimulatorInnerError> {
        let pc = self.unicorn.pc_read().map_err(uc_err)?;
        self.unicorn.emu_start(pc, 0, 0, 1).map_err(uc_err)?;
        sync_regs_from_unicorn(&mut self.unicorn, &mut self.state);
        Ok(())
    }

    fn sync_from(&mut self, dut: &State<P>) {
        self.state.reg.pc = dut.reg.pc;
        self.state.reg.gpr = dut.reg.gpr;
        self.state.reg.fpr = dut.reg.fpr;
        let pc: u64 = (*self.state.reg.pc).into();
        let _ = self.unicorn.set_pc(pc);
        for i in 0..32u32 {
            let val: u64 = self.state.reg.gpr.raw_read(i as usize).into();
            let _ = self.unicorn.reg_write(riscv_reg(i), val);
        }
    }

    fn regs_diff(&self, dut: &State<P>) -> Vec<DifftestMismatchItem> {
        use remu_types::isa::reg::RegDiff;
        let mut out = Vec::new();
        let (r, d) = (&self.state.reg, &dut.reg);
        for (name, ref_val, dut_val) in
            <P::ISA as remu_types::isa::RvIsa>::PcState::diff(&r.pc, &d.pc)
        {
            out.push(DifftestMismatchItem {
                group: RegGroup::Pc,
                name,
                ref_val,
                dut_val,
            });
        }
        for (name, ref_val, dut_val) in
            <P::ISA as remu_types::isa::RvIsa>::GprState::diff(&r.gpr, &d.gpr)
        {
            out.push(DifftestMismatchItem {
                group: RegGroup::Gpr,
                name,
                ref_val,
                dut_val,
            });
        }
        for (name, ref_val, dut_val) in
            <P::ISA as remu_types::isa::RvIsa>::FprState::diff(&r.fpr, &d.fpr)
        {
            out.push(DifftestMismatchItem {
                group: RegGroup::Fpr,
                name,
                ref_val,
                dut_val,
            });
        }
        out
    }

    fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorInnerError> {
        self.state
            .execute(subcmd)
            .map_err(from_state_error)?;
        Ok(())
    }
}

fn riscv_reg(i: u32) -> RegisterRISCV {
    match i {
        0 => RegisterRISCV::X0,
        1 => RegisterRISCV::X1,
        2 => RegisterRISCV::X2,
        3 => RegisterRISCV::X3,
        4 => RegisterRISCV::X4,
        5 => RegisterRISCV::X5,
        6 => RegisterRISCV::X6,
        7 => RegisterRISCV::X7,
        8 => RegisterRISCV::X8,
        9 => RegisterRISCV::X9,
        10 => RegisterRISCV::X10,
        11 => RegisterRISCV::X11,
        12 => RegisterRISCV::X12,
        13 => RegisterRISCV::X13,
        14 => RegisterRISCV::X14,
        15 => RegisterRISCV::X15,
        16 => RegisterRISCV::X16,
        17 => RegisterRISCV::X17,
        18 => RegisterRISCV::X18,
        19 => RegisterRISCV::X19,
        20 => RegisterRISCV::X20,
        21 => RegisterRISCV::X21,
        22 => RegisterRISCV::X22,
        23 => RegisterRISCV::X23,
        24 => RegisterRISCV::X24,
        25 => RegisterRISCV::X25,
        26 => RegisterRISCV::X26,
        27 => RegisterRISCV::X27,
        28 => RegisterRISCV::X28,
        29 => RegisterRISCV::X29,
        30 => RegisterRISCV::X30,
        31 => RegisterRISCV::X31,
        _ => RegisterRISCV::INVALID,
    }
}

fn sync_regs_from_unicorn<P: SimulatorPolicy>(unicorn: &mut Unicorn<'_, ()>, state: &mut State<P>) {
    let pc = unicorn.pc_read().expect("pc_read") as u32;
    *state.reg.pc = pc.into();
    for i in 0..32 {
        let reg = riscv_reg(i);
        let val = unicorn.reg_read(reg).expect("reg_read") as u32;
        state.reg.gpr.raw_write(i as usize, val);
    }
}

fn uc_err(e: unicorn_engine::unicorn_const::uc_error) -> SimulatorInnerError {
    SimulatorInnerError::RefError(e.to_string())
}

//! DPI-C bus_read/bus_write: dispatch via global NZEA pointer to SimulatorNzea.state.
//! RTL is expected to supply **full physical addresses** (same as `remu_state::Bus`), not device-local offsets.

use remu_simulator::{SimulatorCore, SimulatorPolicy};
use remu_state::bus::BusError;

/// Commit info from RTL; pushed by commit_trace DPI, applied after step drains.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CommitMsg {
    pub(crate) next_pc: u32,
    pub(crate) csr_valid: bool,
    pub(crate) csr_addr: u32,
    pub(crate) csr_data: u32,
    pub(crate) gpr_addr: u32,
    pub(crate) gpr_data: u32,
    /// True if the memory access targets an MMIO device.
    pub(crate) is_mmio: bool,
}

pub(crate) trait NzeaDpi {
    fn dpi_read_32(&mut self, addr: usize) -> u32;
    fn dpi_write_32(&mut self, addr: usize, data: u32, wstrb: u32);
    fn dpi_commit_trace(
        &mut self,
        next_pc: u32,
        csr_valid: bool,
        csr_addr: u32,
        csr_data: u32,
        gpr_addr: u32,
        gpr_data: u32,
        is_mmio: bool,
    );
    fn push_commit(&mut self, msg: CommitMsg);
}

impl<P, const IS_DUT: bool> NzeaDpi for crate::SimulatorNzea<P, IS_DUT>
where
    P: SimulatorPolicy + 'static,
    P::ISA: crate::nzea_ffi::NzeaIsa,
{
    fn dpi_read_32(&mut self, addr: usize) -> u32 {
        self.state_mut().bus.read_32(addr).unwrap_or(0)
    }
    fn dpi_write_32(&mut self, addr: usize, data: u32, wstrb: u32) {
        if let Err(BusError::ProgramExit(ec)) =
            self.state_mut().bus.write_32_masked(addr, data, wstrb)
        {
            self.set_pending_exit_code(ec);
        }
    }
    fn dpi_commit_trace(
        &mut self,
        next_pc: u32,
        csr_valid: bool,
        csr_addr: u32,
        csr_data: u32,
        gpr_addr: u32,
        gpr_data: u32,
        is_mmio: bool,
    ) {
        self.push_commit(CommitMsg {
            next_pc,
            csr_valid,
            csr_addr,
            csr_data,
            gpr_addr,
            gpr_data,
            is_mmio,
        });
    }
    fn push_commit(&mut self, msg: CommitMsg) {
        self.push_commit_impl(msg);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FatPtr {
    ptr: *mut (),
    vtable: *mut (),
}

static mut NZEA: FatPtr = FatPtr {
    ptr: std::ptr::null_mut(),
    vtable: std::ptr::null_mut(),
};

pub(crate) unsafe fn set_nzea(nzea: *mut dyn NzeaDpi) {
    unsafe {
        NZEA = std::mem::transmute(nzea);
    }
}

pub(crate) unsafe fn clear_nzea() {
    unsafe {
        NZEA = FatPtr {
            ptr: std::ptr::null_mut(),
            vtable: std::ptr::null_mut(),
        };
    }
}

fn nzea() -> *mut dyn NzeaDpi {
    unsafe { std::mem::transmute(NZEA) }
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn bus_read(addr: i32, rdata: *mut i32) {
    // addr is 32-bit; RISC-V 0x8000_0000 is negative as i32. Preserve bits via u32 to avoid sign-extension.
    let addr_u = addr as u32 as usize;
    let val = unsafe { (*nzea()).dpi_read_32(addr_u) };
    unsafe {
        *rdata = val as i32;
    }
}
#[used]
static _KEEP_BUS_READ: unsafe extern "C" fn(i32, *mut i32) = bus_read;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn bus_write(addr: i32, wdata: i32, wstrb: i32) {
    let addr_u = addr as u32 as usize;
    unsafe {
        (*nzea()).dpi_write_32(addr_u, wdata as u32, wstrb as u32);
    }
}
#[used]
static _KEEP_BUS_WRITE: unsafe extern "C" fn(i32, i32, i32) = bus_write;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn commit_trace(
    next_pc: i32,
    csr_valid: bool,
    csr_addr: i32,
    csr_data: i32,
    gpr_addr: i32,
    gpr_data: i32,
    is_mmio: i32,
) {
    let next_pc_u = next_pc as u32;
    let csr_addr_u = csr_addr as u32;
    let csr_data_u = csr_data as u32;
    let gpr_addr_u = gpr_addr as u32;
    let gpr_data_u = gpr_data as u32;
    let is_mmio_b = is_mmio != 0;
    unsafe {
        (*nzea()).dpi_commit_trace(
            next_pc_u, csr_valid, csr_addr_u, csr_data_u, gpr_addr_u, gpr_data_u, is_mmio_b,
        );
    }
}
#[used]
static _KEEP_COMMIT_TRACE: unsafe extern "C" fn(i32, bool, i32, i32, i32, i32, i32) = commit_trace;

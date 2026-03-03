//! DPI-C bus_read/bus_write: dispatch via global NZEA pointer to SimulatorNzea.state.

use remu_simulator::{SimulatorCore, SimulatorPolicy};

/// Commit info from RTL; pushed by commit_trace DPI, applied after step drains.
#[derive(Clone, Copy, Debug)]
pub struct CommitMsg {
    pub next_pc: u32,
    pub gpr_addr: u32,
    pub gpr_data: u32,
}

pub(crate) trait NzeaDpi {
    fn dpi_read_32(&mut self, addr: usize) -> u32;
    fn dpi_write_32(&mut self, addr: usize, data: u32, wstrb: u32);
    fn dpi_commit_trace(&mut self, next_pc: u32, gpr_addr: u32, gpr_data: u32);
    fn push_commit(&mut self, msg: CommitMsg);
}

impl<P: SimulatorPolicy + 'static, const IS_DUT: bool> NzeaDpi for crate::SimulatorNzea<P, IS_DUT> {
    fn dpi_read_32(&mut self, addr: usize) -> u32 {
        self.state_mut().bus.read_32(addr).unwrap_or(0)
    }
    fn dpi_write_32(&mut self, addr: usize, data: u32, wstrb: u32) {
        for i in 0..4 {
            if (wstrb & (1 << i)) != 0 {
                let _ = self
                    .state_mut()
                    .bus
                    .write_8(addr + i, (data >> (i * 8)) as u8);
            }
        }
    }
    fn dpi_commit_trace(&mut self, next_pc: u32, gpr_addr: u32, gpr_data: u32) {
        self.push_commit(CommitMsg {
            next_pc,
            gpr_addr,
            gpr_data,
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
pub extern "C" fn bus_read(addr: i32, rdata: *mut i32) {
    // addr is 32-bit; RISC-V 0x8000_0000 is negative as i32. Preserve bits via u32 to avoid sign-extension.
    let addr_u = addr as u32 as usize;
    let val = unsafe { (*nzea()).dpi_read_32(addr_u) };
    unsafe {
        *rdata = val as i32;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bus_write(addr: i32, wdata: i32, wstrb: i32) {
    let addr_u = addr as u32 as usize;
    unsafe {
        (*nzea()).dpi_write_32(addr_u, wdata as u32, wstrb as u32);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn commit_trace(next_pc: i32, gpr_addr: i32, gpr_data: i32) {
    let next_pc_u = next_pc as u32;
    let gpr_addr_u = gpr_addr as u32;
    let gpr_data_u = gpr_data as u32;
    unsafe {
        (*nzea()).dpi_commit_trace(next_pc_u, gpr_addr_u, gpr_data_u);
    }
}

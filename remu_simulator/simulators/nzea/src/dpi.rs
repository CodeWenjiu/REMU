//! DPI-C bus_read/bus_write: 通过全局 NZEA 指针调用 SimulatorNzea.state。

use remu_simulator::{SimulatorCore, SimulatorPolicy};

pub(crate) trait NzeaDpi {
    fn dpi_read_32(&mut self, addr: usize) -> u32;
    fn dpi_write_32(&mut self, addr: usize, data: u32, wstrb: u32);
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
    if rdata.is_null() {
        return;
    }
    let val = unsafe { (*nzea()).dpi_read_32(addr as usize) };
    unsafe {
        *rdata = val as i32;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bus_write(addr: i32, wdata: i32, wstrb: i32) {
    unsafe {
        (*nzea()).dpi_write_32(addr as usize, wdata as u32, wstrb as u32);
    }
}

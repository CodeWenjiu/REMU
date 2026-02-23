//! DPI-C bus_read/bus_write for nzea RTL; dispatch via static pointer to State.

use remu_state::State;
use remu_simulator::SimulatorPolicy;

pub(crate) trait DpiBus {
    fn dpi_read_32(&mut self, addr: usize) -> u32;
    fn dpi_write_32(&mut self, addr: usize, data: u32, wstrb: u32);
}

impl<P: SimulatorPolicy> DpiBus for State<P> {
    fn dpi_read_32(&mut self, addr: usize) -> u32 {
        self.bus.read_32(addr).unwrap_or(0)
    }
    fn dpi_write_32(&mut self, addr: usize, data: u32, wstrb: u32) {
        for i in 0..4 {
            if (wstrb & (1 << i)) != 0 {
                let _ = self.bus.write_8(addr + i, (data >> (i * 8)) as u8);
            }
        }
    }
}

static mut DPI_BUS: Option<*mut dyn DpiBus> = None;

pub(crate) unsafe fn set_dpi_bus(bus: *mut dyn DpiBus) {
    unsafe { DPI_BUS = Some(bus); }
}
pub(crate) unsafe fn clear_dpi_bus() {
    unsafe { DPI_BUS = None; }
}

#[unsafe(no_mangle)]
pub extern "C" fn bus_read(addr: i32, rdata: *mut i32) {
    if rdata.is_null() {
        return;
    }
    let val = unsafe {
        DPI_BUS
            .and_then(|p| p.as_mut())
            .map(|b| b.dpi_read_32(addr as usize))
            .unwrap_or(0)
    };
    unsafe {
        *rdata = val as i32;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bus_write(addr: i32, wdata: i32, wstrb: i32) {
    unsafe {
        if let Some(b) = DPI_BUS.and_then(|p| p.as_mut()) {
            b.dpi_write_32(addr as usize, wdata as u32, wstrb as u32);
        }
    }
}

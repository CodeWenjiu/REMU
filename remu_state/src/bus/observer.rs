pub trait BusObserver {
    #[inline(always)]
    fn on_mem_write(&mut self, addr: usize, len: usize, val: u64) {
        let _ = (addr, len, val);
    }

    #[inline(always)]
    fn on_mmio_read(&mut self, addr: usize, val: u64) {
        let _ = (addr, val);
    }

    #[inline(always)]
    fn on_mmio_write(&mut self, addr: usize, val: u64) {
        let _ = (addr, val);
    }
}

impl BusObserver for () {}

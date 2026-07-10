#[derive(Debug, Clone)]
pub enum ObserverEvent {
    /// MMIO was accessed this step; harness should sync ref and skip difftest.
    MmioAccess,
}

pub trait BusObserver {
    const ENABLED: bool = true;

    fn new() -> Self;

    #[inline(always)]
    fn on_mem_read_8(&mut self, addr: usize, val: u8) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mem_read_16(&mut self, addr: usize, val: u16) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mem_read_32(&mut self, addr: usize, val: u32) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mem_read_64(&mut self, addr: usize, val: u64) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mem_read_128(&mut self, addr: usize, val: u128) {
        let _ = (addr, val);
    }

    #[inline(always)]
    fn on_mem_write_8(&mut self, addr: usize, val: u8) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mem_write_16(&mut self, addr: usize, val: u16) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mem_write_32(&mut self, addr: usize, val: u32) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mem_write_64(&mut self, addr: usize, val: u64) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mem_write_128(&mut self, addr: usize, val: u128) {
        let _ = (addr, val);
    }

    #[inline(always)]
    fn on_mmio_read_8(&mut self, addr: usize, val: u8) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mmio_read_16(&mut self, addr: usize, val: u16) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mmio_read_32(&mut self, addr: usize, val: u32) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mmio_read_64(&mut self, addr: usize, val: u64) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mmio_read_128(&mut self, addr: usize, val: u128) {
        let _ = (addr, val);
    }

    #[inline(always)]
    fn on_mmio_write_8(&mut self, addr: usize, val: u8) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mmio_write_16(&mut self, addr: usize, val: u16) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mmio_write_32(&mut self, addr: usize, val: u32) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mmio_write_64(&mut self, addr: usize, val: u64) {
        let _ = (addr, val);
    }
    #[inline(always)]
    fn on_mmio_write_128(&mut self, addr: usize, val: u128) {
        let _ = (addr, val);
    }

    #[inline(always)]
    fn get_events_and_clear(&mut self) -> Vec<ObserverEvent> {
        vec![]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FastObserver;

impl BusObserver for FastObserver {
    const ENABLED: bool = false;

    fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone)]
pub struct DifftestObserver {
    events: Vec<ObserverEvent>,
}

impl BusObserver for DifftestObserver {
    fn new() -> Self {
        Self { events: Vec::new() }
    }

    fn on_mmio_read_8(&mut self, addr: usize, val: u8) {
        let _ = (addr, val);
        self.events.push(ObserverEvent::MmioAccess);
    }
    fn on_mmio_read_16(&mut self, addr: usize, val: u16) {
        let _ = (addr, val);
        self.events.push(ObserverEvent::MmioAccess);
    }
    fn on_mmio_read_32(&mut self, addr: usize, val: u32) {
        let _ = (addr, val);
        self.events.push(ObserverEvent::MmioAccess);
    }
    fn on_mmio_read_64(&mut self, addr: usize, val: u64) {
        let _ = (addr, val);
        self.events.push(ObserverEvent::MmioAccess);
    }
    fn on_mmio_read_128(&mut self, addr: usize, val: u128) {
        let _ = (addr, val);
        self.events.push(ObserverEvent::MmioAccess);
    }

    fn on_mmio_write_8(&mut self, addr: usize, val: u8) {
        let _ = (addr, val);
        self.events.push(ObserverEvent::MmioAccess);
    }
    fn on_mmio_write_16(&mut self, addr: usize, val: u16) {
        let _ = (addr, val);
        self.events.push(ObserverEvent::MmioAccess);
    }
    fn on_mmio_write_32(&mut self, addr: usize, val: u32) {
        let _ = (addr, val);
        self.events.push(ObserverEvent::MmioAccess);
    }
    fn on_mmio_write_64(&mut self, addr: usize, val: u64) {
        let _ = (addr, val);
        self.events.push(ObserverEvent::MmioAccess);
    }
    fn on_mmio_write_128(&mut self, addr: usize, val: u128) {
        let _ = (addr, val);
        self.events.push(ObserverEvent::MmioAccess);
    }

    fn get_events_and_clear(&mut self) -> Vec<ObserverEvent> {
        std::mem::take(&mut self.events)
    }
}

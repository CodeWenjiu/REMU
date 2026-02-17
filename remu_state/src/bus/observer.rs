#[derive(Debug, Clone)]
pub enum ObserverEvent {
    /// MMIO was accessed this step; harness should sync ref and skip difftest.
    MmioAccess,
    /// One memory write (to RAM): (start_addr, data). Used for memdiff. Fixed length, so Box<[u8]>.
    MemoryWrite(usize, Box<[u8]>),
}

pub trait BusObserver {
    /// Indicates whether this observer is enabled.
    ///
    /// # Performance Optimization
    ///
    /// This constant is primarily used to assist the compiler with **Dead Code Elimination (DCE)**.
    ///
    /// Even if a trait method is empty (e.g., `impl BusObserver for ()`), without full inlining,
    /// the compiler is strictly bound by the **ABI calling convention**. This forces the allocation
    /// of a general-purpose register (often RCX or RDX on x86_64) to pass the `&mut self` pointer.
    ///
    /// In tight simulation loops, this unnecessary register pressure causes **Register Spilling**
    /// (forcing other hot variables onto the stack), leading to a 5-10% performance regression.
    ///
    /// By explicitly checking `if O::ENABLED`, we leverage Rust's **Constant Propagation**.
    /// The compiler detects the unreachable code path at the MIR level and completely removes
    /// the function call branch, ensuring zero overhead for inactive observers.
    ///
    /// # Default
    /// Defaults to `true`. This should be overridden to `false` for zero-cost observers
    /// (like `()`) to guarantee that no assembly instructions are generated.
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

    /// Take and clear all events this step (MMIO and/or memory writes). Default: empty.
    #[inline(always)]
    fn get_events_and_clear(&mut self) -> Vec<ObserverEvent> {
        vec![]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FastObserver;

impl BusObserver for FastObserver {
    // Explicitly disable to trigger Dead Code Elimination.
    const ENABLED: bool = false;

    fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone)]
pub struct DifftestObserver {
    /// Events this step: MemoryWrite(addr, data) and/or MmioAccess.
    events: Vec<ObserverEvent>,
}

impl BusObserver for DifftestObserver {
    fn new() -> Self {
        Self {
            events: Vec::new(),
        }
    }

    fn on_mem_write_8(&mut self, addr: usize, val: u8) {
        self.events
            .push(ObserverEvent::MemoryWrite(addr, vec![val].into_boxed_slice()));
    }
    fn on_mem_write_16(&mut self, addr: usize, val: u16) {
        self.events
            .push(ObserverEvent::MemoryWrite(addr, Box::from(val.to_le_bytes())));
    }
    fn on_mem_write_32(&mut self, addr: usize, val: u32) {
        self.events
            .push(ObserverEvent::MemoryWrite(addr, Box::from(val.to_le_bytes())));
    }
    fn on_mem_write_64(&mut self, addr: usize, val: u64) {
        self.events
            .push(ObserverEvent::MemoryWrite(addr, Box::from(val.to_le_bytes())));
    }
    fn on_mem_write_128(&mut self, addr: usize, val: u128) {
        self.events
            .push(ObserverEvent::MemoryWrite(addr, Box::from(val.to_le_bytes())));
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

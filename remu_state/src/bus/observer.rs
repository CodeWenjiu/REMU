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

#[derive(Debug, Clone, Copy)]
pub struct MmioObserver {
    pub is_modified: bool,
}

impl BusObserver for MmioObserver {
    fn new() -> Self {
        Self { is_modified: false }
    }

    fn on_mmio_read_8(&mut self, addr: usize, val: u8) {
        let _ = (addr, val);
        self.is_modified = true;
    }
    fn on_mmio_read_16(&mut self, addr: usize, val: u16) {
        let _ = (addr, val);
        self.is_modified = true;
    }
    fn on_mmio_read_32(&mut self, addr: usize, val: u32) {
        let _ = (addr, val);
        self.is_modified = true;
    }
    fn on_mmio_read_64(&mut self, addr: usize, val: u64) {
        let _ = (addr, val);
        self.is_modified = true;
    }
    fn on_mmio_read_128(&mut self, addr: usize, val: u128) {
        let _ = (addr, val);
        self.is_modified = true;
    }

    fn on_mmio_write_8(&mut self, addr: usize, val: u8) {
        let _ = (addr, val);
        self.is_modified = true;
    }
    fn on_mmio_write_16(&mut self, addr: usize, val: u16) {
        let _ = (addr, val);
        self.is_modified = true;
    }
    fn on_mmio_write_32(&mut self, addr: usize, val: u32) {
        let _ = (addr, val);
        self.is_modified = true;
    }
    fn on_mmio_write_64(&mut self, addr: usize, val: u64) {
        let _ = (addr, val);
        self.is_modified = true;
    }
    fn on_mmio_write_128(&mut self, addr: usize, val: u128) {
        let _ = (addr, val);
        self.is_modified = true;
    }
}

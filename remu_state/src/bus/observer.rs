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

impl BusObserver for () {
    // Explicitly disable to trigger Dead Code Elimination.
    const ENABLED: bool = false;
}

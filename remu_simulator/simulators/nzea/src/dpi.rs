//! DPI-C bus/commit entry points: **width-typed**, one channel per ISA word.
//!
//! The RTL calls `bus_read` / `bus_write` / `commit_trace`; the C bridge in
//! `nzea_wrapper.cpp` (compiled per model, `-DNZEA_XLEN=32|64`) forwards
//! same-width to the `nzea_dpi_*` symbols exported here. Everything is typed by
//! the ISA's machine word `W` (`u32` or `u64`): [`CommitMsg<W>`] and the
//! [`NzeaDpi<W>`] methods carry `W` directly, so an RV32 run never touches a
//! 64-bit value and vice versa.

use remu_isa::isa::RvIsa;
use remu_simulator::{SimulatorCore, SimulatorPolicy};
use remu_state::StatePolicy;
use remu_state::bus::{BusError, BusWord};

use crate::nzea_ffi::NzeaIsa;

/// Commit info from RTL; pushed by commit_trace DPI, applied after step drains.
/// Every payload field is the ISA word; the two addresses are register indices.
#[derive(Clone, Copy, Debug)]
pub struct CommitMsg<W: BusWord> {
    pub(crate) next_pc: W,
    pub(crate) csr_valid: bool,
    pub(crate) csr_addr: u16,
    pub(crate) csr_data: W,
    pub(crate) gpr_addr: u8,
    pub(crate) gpr_data: W,
    /// True if the memory access targets an MMIO device.
    pub(crate) is_mmio: bool,
}

/// The DPI boundary for one ISA word width. `--isa` selects a single ISA per
/// run, so at most one instance is live in one width channel.
pub trait NzeaDpi<W: BusWord> {
    /// Read one XLEN-wide bus word at `addr`.
    fn dpi_read(&mut self, addr: usize) -> W;
    /// Masked write of one XLEN-wide bus word; `wstrb` bit i enables byte i.
    fn dpi_write(&mut self, addr: usize, data: W, wstrb: u32);
    fn dpi_commit_trace(&mut self, msg: CommitMsg<W>);
}

impl<P, const IS_DUT: bool, W> NzeaDpi<W> for crate::SimulatorNzea<P, IS_DUT>
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa + RvIsa<XLEN = W>,
    W: BusWord,
{
    #[inline(always)]
    fn dpi_read(&mut self, addr: usize) -> W {
        self.state_mut()
            .bus
            .read_word::<W>(addr)
            .unwrap_or_default()
    }

    #[inline(always)]
    fn dpi_write(&mut self, addr: usize, data: W, wstrb: u32) {
        let res = self
            .state_mut()
            .bus
            .write_word_masked::<W>(addr, data, wstrb);
        if let Err(BusError::ProgramExit(ec)) = res {
            self.set_pending_exit_code(ec);
        }
    }

    fn dpi_commit_trace(&mut self, msg: CommitMsg<W>) {
        self.push_commit_impl(msg);
    }
}

/// Width-selected DPI channel: where the live instance pointer for one machine
/// word width lives. Implemented for the two bus words only, so a width that is
/// neither 32 nor 64 bits cannot reach the DPI boundary.
pub trait DpiChannel: BusWord {
    fn set(inst: *mut dyn NzeaDpi<Self>);
    /// `None` before `set` and after `clear`; the exported shims are only called
    /// while a model is running, but they tolerate the empty channel.
    fn get() -> Option<*mut dyn NzeaDpi<Self>>;
    fn clear();
}

static mut NZEA32: Option<*mut dyn NzeaDpi<u32>> = None;
static mut NZEA64: Option<*mut dyn NzeaDpi<u64>> = None;

impl DpiChannel for u32 {
    #[inline(always)]
    fn set(inst: *mut dyn NzeaDpi<u32>) {
        unsafe { NZEA32 = Some(inst) }
    }
    #[inline(always)]
    fn get() -> Option<*mut dyn NzeaDpi<u32>> {
        unsafe { NZEA32 }
    }
    #[inline(always)]
    fn clear() {
        unsafe { NZEA32 = None }
    }
}

impl DpiChannel for u64 {
    #[inline(always)]
    fn set(inst: *mut dyn NzeaDpi<u64>) {
        unsafe { NZEA64 = Some(inst) }
    }
    #[inline(always)]
    fn get() -> Option<*mut dyn NzeaDpi<u64>> {
        unsafe { NZEA64 }
    }
    #[inline(always)]
    fn clear() {
        unsafe { NZEA64 = None }
    }
}

/// The ISA word of a policy (`u32`/`u64`): the type a DPI channel is keyed by.
type WordOf<P> = <<P as StatePolicy>::ISA as RvIsa>::XLEN;

/// Install the live instance into its XLEN channel.
///
/// The instance pointer is only moved in `new`/`init`; `step_once` re-installs
/// it because the initial address (`Harness::new`'s local) goes stale once the
/// simulator is moved into the harness.
pub(crate) fn set_nzea<P, const IS_DUT: bool>(nzea: *mut crate::SimulatorNzea<P, IS_DUT>)
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa,
{
    <WordOf<P> as DpiChannel>::set(nzea as *mut dyn NzeaDpi<WordOf<P>>);
}

pub(crate) fn clear_nzea<P, const IS_DUT: bool>()
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa,
{
    <WordOf<P> as DpiChannel>::clear();
}

// ── Exported DPI shims ──
//
// One trio per width, called by the same-width C bridge. Addresses and data are
// already the ISA word (`u32`/`u64`), so nothing is widened or narrowed here.

#[unsafe(no_mangle)]
pub(crate) extern "C" fn nzea_dpi_read32(addr: u32, rdata: *mut u32) {
    let val = <u32 as DpiChannel>::get()
        .map(|p| unsafe { (*p).dpi_read(addr as usize) })
        .unwrap_or_default();
    unsafe {
        *rdata = val;
    }
}
#[used]
static _KEEP_READ32: unsafe extern "C" fn(u32, *mut u32) = nzea_dpi_read32;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn nzea_dpi_write32(addr: u32, wdata: u32, wstrb: u32) {
    if let Some(p) = <u32 as DpiChannel>::get() {
        unsafe { (*p).dpi_write(addr as usize, wdata, wstrb) };
    }
}
#[used]
static _KEEP_WRITE32: unsafe extern "C" fn(u32, u32, u32) = nzea_dpi_write32;

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub(crate) extern "C" fn nzea_dpi_commit32(
    next_pc: u32,
    csr_valid: bool,
    csr_addr: u32,
    csr_data: u32,
    gpr_addr: u32,
    gpr_data: u32,
    is_mmio: bool,
) {
    unsafe {
        if let Some(p) = <u32 as DpiChannel>::get() {
            (*p).dpi_commit_trace(CommitMsg {
                next_pc,
                csr_valid,
                csr_addr: csr_addr as u16,
                csr_data,
                gpr_addr: gpr_addr as u8,
                gpr_data,
                is_mmio,
            });
        }
    }
}
#[used]
static _KEEP_COMMIT32: unsafe extern "C" fn(u32, bool, u32, u32, u32, u32, bool) =
    nzea_dpi_commit32;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn nzea_dpi_read64(addr: u64, rdata: *mut u64) {
    let val = <u64 as DpiChannel>::get()
        .map(|p| unsafe { (*p).dpi_read(addr as usize) })
        .unwrap_or_default();
    unsafe {
        *rdata = val;
    }
}
#[used]
static _KEEP_READ64: unsafe extern "C" fn(u64, *mut u64) = nzea_dpi_read64;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn nzea_dpi_write64(addr: u64, wdata: u64, wstrb: u32) {
    if let Some(p) = <u64 as DpiChannel>::get() {
        unsafe { (*p).dpi_write(addr as usize, wdata, wstrb) };
    }
}
#[used]
static _KEEP_WRITE64: unsafe extern "C" fn(u64, u64, u32) = nzea_dpi_write64;

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub(crate) extern "C" fn nzea_dpi_commit64(
    next_pc: u64,
    csr_valid: bool,
    csr_addr: u32,
    csr_data: u64,
    gpr_addr: u32,
    gpr_data: u64,
    is_mmio: bool,
) {
    unsafe {
        if let Some(p) = <u64 as DpiChannel>::get() {
            (*p).dpi_commit_trace(CommitMsg {
                next_pc,
                csr_valid,
                csr_addr: csr_addr as u16,
                csr_data,
                gpr_addr: gpr_addr as u8,
                gpr_data,
                is_mmio,
            });
        }
    }
}
#[used]
static _KEEP_COMMIT64: unsafe extern "C" fn(u64, bool, u32, u64, u32, u64, bool) =
    nzea_dpi_commit64;

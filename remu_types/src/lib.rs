remu_macro::mod_pub!(prelude);
remu_macro::mod_flat!(difftest, exit_code, platform, trace_flags);

// Re-export from remu_isa (backward compat; new code should use remu_isa directly)
pub use remu_isa::{AllUsize, Xlen, isa};

use std::{cell::RefCell, error::Error, ops::Range, rc::Rc};

use remu_isa::isa::reg::Gpr;

pub trait DynDiagError: Error {}
impl<T> DynDiagError for T where T: Error {}

pub trait Tracer {
    fn print(&self, message: &str);

    fn mem_print(&self, begin: usize, data: &[u8], result: Result<(), Box<dyn DynDiagError>>);
    fn mem_show(&self, begin: usize, data: Result<AllUsize, Box<dyn DynDiagError>>);
    fn mem_show_map(&self, map: Vec<(String, Range<usize>)>);

    fn reg_print(&self, regs: &[(Gpr, u32); 32], range: Range<usize>);
    fn reg_show(&self, index: Gpr, data: u32);

    fn reg_show_pc(&self, data: u32) {
        let _ = data;
    }
    fn reg_show_fpr(&self, index: usize, data: u32) {
        let _ = (index, data);
    }
    fn reg_print_fpr(&self, regs: &[(usize, u32)], range: Range<usize>) {
        let _ = (regs, range);
    }

    fn reg_show_vr(&self, index: usize, data: &[u8]) {
        let _ = (index, data);
    }
    fn reg_print_vr(&self, regs: &[(usize, Vec<u8>)], range: Range<usize>) {
        let _ = (regs, range);
    }

    fn disasm(&self, pc: u64, inst: u32);

    fn breakpoint_print(&self, addrs: &[u32]) {
        let _ = addrs;
    }

    fn stat_print(&self, _entries: &[(String, String)]) {}
}

pub type TracerDyn = Rc<RefCell<dyn Tracer>>;

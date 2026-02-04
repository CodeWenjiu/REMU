remu_macro::mod_pub!(isa);
remu_macro::mod_flat!(difftest, platform, wordlen);

use std::{cell::RefCell, error::Error, fmt::Display, ops::Range, rc::Rc};

use crate::isa::reg::Gpr;
pub trait DynDiagError: Error {}
impl<T> DynDiagError for T where T: Error {}

#[derive(Debug, Clone)]
pub enum AllUsize {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
}

impl Display for AllUsize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AllUsize::U8(v) => write!(f, "0x{:02x}", v),
            AllUsize::U16(v) => write!(f, "0x{:04x}", v),
            AllUsize::U32(v) => write!(f, "0x{:08x}", v),
            AllUsize::U64(v) => write!(f, "0x{:016x}", v),
            AllUsize::U128(v) => write!(f, "0x{:032x}", v),
        }
    }
}

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

    fn disasm(&self, pc: u64, inst: u32);
}

pub type TracerDyn = Rc<RefCell<dyn Tracer>>;

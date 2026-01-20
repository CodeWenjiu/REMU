use std::{cell::RefCell, error::Error, fmt::Display, rc::Rc};
pub trait DynDiagError: Error {}
impl<T> DynDiagError for T where T: Error {}

#[derive(Debug)]
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
    fn mem_print(&self, begin: usize, data: &[u8], result: Result<(), Box<dyn DynDiagError>>);
    fn mem_show(&self, begin: usize, data: Result<AllUsize, Box<dyn DynDiagError>>);

    fn deal_error(&self, error: Box<dyn DynDiagError>);
}

pub type TracerDyn = Rc<RefCell<dyn Tracer>>;

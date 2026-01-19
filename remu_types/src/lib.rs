use std::{cell::RefCell, error::Error, rc::Rc};

pub trait Tracer {
    fn mem_print(&self, begin: u64, data: &[u8], result: Result<(), Box<dyn Error>>);
}

pub type TracerDyn = Rc<RefCell<dyn Tracer>>;

use std::{cell::RefCell, rc::Rc};

pub trait Tracer {
    fn mem_print(&self, begin: u64, data: u64);
}

pub type TracerDyn = Rc<RefCell<dyn Tracer>>;

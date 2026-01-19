use std::{cell::RefCell, error::Error, rc::Rc};

pub trait DynDiagError: Error {}
impl<T> DynDiagError for T where T: Error {}

pub trait Tracer {
    fn mem_print(&self, begin: usize, data: &[u8], result: Result<(), Box<dyn DynDiagError>>);
    fn deal_error(&self, error: Box<dyn DynDiagError>);
}

pub type TracerDyn = Rc<RefCell<dyn Tracer>>;

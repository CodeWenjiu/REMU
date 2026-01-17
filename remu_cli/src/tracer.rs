use remu_types::Tracer;

pub struct CLITracer;

impl Tracer for CLITracer {
    fn mem_print(&self, begin: u64, data: u64) {
        println!("begin: {}, data: {}", begin, data);
    }
}

impl CLITracer {
    pub fn new() -> Self {
        CLITracer
    }
}

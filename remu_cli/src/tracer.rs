use remu_types::Tracer;

pub struct CLITracer;

impl Tracer for CLITracer {
    fn mem_print(&self, begin: usize, data: &[u8], result: Result<(), Box<dyn std::error::Error>>) {
        print!("begin: {} ", begin);
        match result {
            Ok(_) => println!("Value: {:?}", data),
            Err(err) => println!("Error: {}", err),
        }
    }
}

impl CLITracer {
    pub fn new() -> Self {
        CLITracer
    }
}

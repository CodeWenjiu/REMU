use remu_types::DynDiagError;

use crate::bus::{Bus, BusAccess, BusOption};

remu_macro::mod_pub!(bus);
remu_macro::mod_flat!(commands);

/// State template
pub struct State {
    pub bus: Bus,
    tracer: remu_types::TracerDyn,
}

impl State {
    pub fn new(opt: StateOption, tracer: remu_types::TracerDyn) -> Self {
        Self {
            bus: Bus::new(opt.bus),
            tracer,
        }
    }

    pub fn execute(&mut self, subcmd: &StateCmds) {
        match subcmd {
            StateCmds::Hello => tracing::info!("hello state"),
            StateCmds::Print { start, count } => {
                let mut buf = vec![0u8 as u8; *count];
                let result = self
                    .bus
                    .read_bytes(*start, &mut buf)
                    .map_err(|e| Box::new(e) as Box<dyn DynDiagError>);
                self.tracer.borrow_mut().mem_print(*start, &buf, result);
            }
            StateCmds::Set { address, value } => {
                let data: Vec<u8> = value.iter().flat_map(|v| v.iter().copied()).collect();
                if let Err(e) = self.bus.write_bytes(*address, &data) {
                    self.tracer
                        .borrow()
                        .deal_error(Box::new(e) as Box<dyn DynDiagError>);
                }
            }
        }
    }
}

#[derive(clap::Args, Debug)]
pub struct StateOption {
    /// Bus Option
    #[command(flatten)]
    pub bus: BusOption,
}

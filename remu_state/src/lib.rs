use remu_types::{AllUsize, DynDiagError};

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
            StateCmds::Read { subcmd } => match subcmd {
                ReadCommand::U8(arg) => self.tracer.borrow().mem_show(
                    arg.addr,
                    self.bus
                        .read_8(arg.addr)
                        .map(|v| AllUsize::U8(v))
                        .map_err(|e| e as Box<dyn DynDiagError>),
                ),
                ReadCommand::U16(arg) => self.tracer.borrow().mem_show(
                    arg.addr,
                    self.bus
                        .read_16(arg.addr)
                        .map(|v| AllUsize::U16(v))
                        .map_err(|e| e as Box<dyn DynDiagError>),
                ),
                ReadCommand::U32(arg) => self.tracer.borrow().mem_show(
                    arg.addr,
                    self.bus
                        .read_32(arg.addr)
                        .map(|v| AllUsize::U32(v))
                        .map_err(|e| e as Box<dyn DynDiagError>),
                ),
                ReadCommand::U64(arg) => self.tracer.borrow().mem_show(
                    arg.addr,
                    self.bus
                        .read_64(arg.addr)
                        .map(|v| AllUsize::U64(v))
                        .map_err(|e| e as Box<dyn DynDiagError>),
                ),
                ReadCommand::U128(arg) => self.tracer.borrow().mem_show(
                    arg.addr,
                    self.bus
                        .read_128(arg.addr)
                        .map(|v| AllUsize::U128(v))
                        .map_err(|e| e as Box<dyn DynDiagError>),
                ),
            },
            StateCmds::Print { addr, count } => {
                let mut buf = vec![0u8 as u8; *count];
                let result = self
                    .bus
                    .read_bytes(*addr, &mut buf)
                    .map_err(|e| e as Box<dyn DynDiagError>);
                self.tracer.borrow_mut().mem_print(*addr, &buf, result);
            }
            StateCmds::Write { subcmd } => match subcmd {
                WriteCommand::U8 { addr, value } => {
                    if let Err(e) = self.bus.write_8(*addr, *value) {
                        self.tracer.borrow().deal_error(e as Box<dyn DynDiagError>);
                    }
                }
                WriteCommand::U16 { addr, value } => {
                    if let Err(e) = self.bus.write_16(*addr, *value) {
                        self.tracer.borrow().deal_error(e as Box<dyn DynDiagError>);
                    }
                }
                WriteCommand::U32 { addr, value } => {
                    if let Err(e) = self.bus.write_32(*addr, *value) {
                        self.tracer.borrow().deal_error(e as Box<dyn DynDiagError>);
                    }
                }
                WriteCommand::U64 { addr, value } => {
                    if let Err(e) = self.bus.write_64(*addr, *value) {
                        self.tracer.borrow().deal_error(e as Box<dyn DynDiagError>);
                    }
                }
                WriteCommand::U128 { addr, value } => {
                    if let Err(e) = self.bus.write_128(*addr, *value) {
                        self.tracer.borrow().deal_error(e as Box<dyn DynDiagError>);
                    }
                }
            },
            StateCmds::Set { address, value } => {
                let mut addr = *address;
                for chunk in value.iter() {
                    if chunk.is_empty() {
                        continue;
                    }
                    if let Err(e) = self.bus.write_bytes(addr, chunk) {
                        self.tracer.borrow().deal_error(e as Box<dyn DynDiagError>);
                        break;
                    }
                    addr = addr.saturating_add(chunk.len());
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

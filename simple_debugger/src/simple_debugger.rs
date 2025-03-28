use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::{BaseConfiguration, DebugConfiguration, OptionParser};
use remu_buildin::{get_buildin_img, get_reset_vector, READLINE_HISTORY_LENGTH};
use remu_macro::log_err;
use simulator::{difftest_ref::difftestffi_init, Simulator};
use state::States;
use crate::cmd_parser::Server;

use remu_utils::{DifftestRef, Disassembler, ProcessError};

pub struct SimpleDebugger {
    server: Server,

    pub disassembler: Rc<RefCell<Disassembler>>,

    pub state: States,
    pub state_ref: States,

    pub simulator: Simulator,
}

impl SimpleDebugger {
    pub fn new(cli_result: OptionParser) -> Result<Self, ()> {
        let disassembler = Disassembler::new(cli_result.cli.platform.isa)?;
        let disassembler = Rc::new(RefCell::new(disassembler));

        if let Some(difftest_ref) = cli_result.cli.differtest {
            Logger::function(&format!("differtest \"{}\"", difftest_ref).to_string(), true);
        } else {
            Logger::function("differtest", false);
        }

        let rl_history_length = cli_result.cfg.debug_config.iter()
            .find_map(|config| {
                if let DebugConfiguration::Readline { history } = config {
                    Some(*history)
                } else {
                    None
                }
            })
            .unwrap_or(READLINE_HISTORY_LENGTH);

        let (state, state_ref) = Self::state_init(&cli_result);

        let simulator = log_err!(Simulator::new(&cli_result, state.clone(), state_ref.clone(), disassembler.clone()))?;

        Ok(Self {
            server: Server::new(cli_result.cli.platform.simulator, rl_history_length).expect("Unable to create server"),

            disassembler,

            state,
            state_ref,
            
            simulator,
        })
    }

    fn state_init(cli_result: &OptionParser) -> (States, States) {
        let isa = cli_result.cli.platform.isa;

        let mut reset_vector = get_reset_vector(isa);

        for base_config in &cli_result.cfg.base_config {
            match base_config {
                BaseConfiguration::ResetVector { value } => {
                    reset_vector = *value;
                }
            }
        }

        let mut state = States::new(isa, reset_vector).unwrap();
        let mut state_ref = state.clone();

        if let Some(DifftestRef::BuildIn(_)) = cli_result.cli.differtest {
            state_ref = States::new(isa, reset_vector).unwrap();
        }

        for region in &cli_result.cfg.region_config {
            log_err!(state.mmu.add_region(region.base, region.size, &region.name, region.flag.clone(), region.r#type.clone())).unwrap();
            
            if let Some(DifftestRef::BuildIn(_)) = cli_result.cli.differtest {
                log_err!(state_ref.mmu.add_region(region.base, region.size, &region.name, region.flag.clone(), region.r#type.clone())).unwrap();
            }
        }

        let buildin_img = get_buildin_img(isa);

        let bytes = if cli_result.cli.bin.is_some() {
            let bin = cli_result.cli.bin.as_ref().unwrap();
            let bytes = log_err!(std::fs::read(bin)).unwrap();
            
            Logger::show(&format!("Loading binary image size: {}", bytes.len() / 4).to_string(), Logger::INFO);

            bytes
        } else {
            let bytes: Vec<u8> = buildin_img.iter()
                .flat_map(|&val| val.to_le_bytes().to_vec())
                .collect();
    
            Logger::show("No binary image specified, using buildin image.", Logger::WARN);

            bytes
        };
        
        log_err!(state.mmu.load(reset_vector, &bytes)).unwrap();
        if let Some(DifftestRef::BuildIn(_)) = cli_result.cli.differtest {
            log_err!(state_ref.mmu.load(reset_vector, &bytes)).unwrap();
        } else {
            difftestffi_init(&state.regfile, bytes, reset_vector);
        }

        if cli_result.cli.differtest.is_none() {
            state_ref = state.clone();
        }

        (state, state_ref)
    }

    pub fn mainloop(mut self) -> Result<(), ()> {
        loop {
            macro_rules! handle_result {
                ($result:expr) => {
                    match $result {
                        Err(ProcessError::Recoverable) => continue,
                        Err(ProcessError::GracefulExit) => return Ok(()),
                        Err(ProcessError::Fatal) => return Err(()),
                        Ok(value) => value,
                    }
                };
            }

            let cmd = handle_result!(self.server.get_parse());
            handle_result!(self.execute(cmd.command));
        }
    }
}

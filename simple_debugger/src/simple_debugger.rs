use std::{cell::RefCell, rc::Rc};

use crate::cmd_parser::Server;
use logger::Logger;
use option_parser::OptionParser;
use remu_buildin::get_buildin_img;
use remu_macro::log_err;
use simulator::{Simulator, difftest_ref::difftestffi_init};
use state::{model::StageModel, States};

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
            Logger::function(
                &format!("differtest \"{}\"", difftest_ref).to_string(),
                true,
            );
        } else {
            Logger::function("differtest", false);
        }

        let rl_history_length = cli_result
            .cfg
            .debug_config
            .rl_history_size;

        let (state, state_ref) = Self::state_init(&cli_result);

        let simulator = log_err!(Simulator::new(
            &cli_result,
            state.clone(),
            state_ref.clone(),
            disassembler.clone()
        ))?;

        Ok(Self {
            server: Server::new(cli_result.cli.platform.simulator, rl_history_length)
                .expect("Unable to create server"),

            disassembler,

            state,
            state_ref,

            simulator,
        })
    }

    fn state_init(cli_result: &OptionParser) -> (States, States) {
        let isa = cli_result.cli.platform.isa;

        let reset_vector = cli_result.cfg.platform_config.reset_vector;

        let mut state = States::new(isa, reset_vector, StageModel::default()).unwrap();
        let mut state_ref = state.clone();

        if let Some(DifftestRef::BuildIn(_)) = cli_result.cli.differtest {
            state_ref = States::new(isa, reset_vector, StageModel::default()).unwrap();
        }

        for region in &cli_result.cfg.platform_config.regions {
            log_err!(state.mmu.add_region(
                region.base,
                region.size,
                &region.name,
                region.flag.clone(),
                region.mmtype
            ))
            .unwrap();

            if let Some(DifftestRef::BuildIn(_)) = cli_result.cli.differtest {
                log_err!(state_ref.mmu.add_region(
                    region.base,
                    region.size,
                    &region.name,
                    region.flag.clone(),
                    region.mmtype
                ))
                .unwrap();
            }
        }

        if cli_result.cli.additional_bin.is_some() {
            let bin = cli_result.cli.additional_bin.as_ref().unwrap();

            let bin_path = &bin.file_path;
            let bytes = log_err!(std::fs::read(bin_path)).unwrap();
            log_err!(state.mmu.load(bin.load_addr, &bytes)).unwrap();

            match cli_result.cli.differtest {
                Some(DifftestRef::BuildIn(_)) => {
                    log_err!(state_ref.mmu.load(0x80100000, &bytes)).unwrap();
                }

                _ => ()
            }
        };

        let buildin_img = get_buildin_img(isa);

        let bytes = if cli_result.cli.primary_bin.is_some() {
            let bin = cli_result.cli.primary_bin.as_ref().unwrap();
            let bytes = log_err!(std::fs::read(bin))
                .map_err(|e| {
                    Logger::show(
                        &format!("Unable to read binary image {}", bin).to_string(),
                        Logger::ERROR,
                    );
                    e
                })
                .unwrap();

            Logger::show(
                &format!("Loading binary image {} size: {}", bin, bytes.len() / 4).to_string(),
                Logger::INFO,
            );

            bytes
        } else {
            let bytes: Vec<u8> = buildin_img
                .iter()
                .flat_map(|&val| val.to_le_bytes().to_vec())
                .collect();

            Logger::show(
                "No binary image specified, using buildin image.",
                Logger::WARN,
            );

            bytes
        };

        log_err!(state.mmu.load(reset_vector, &bytes)).unwrap();

        match cli_result.cli.differtest {
            Some(DifftestRef::BuildIn(_)) => {
                log_err!(state_ref.mmu.load(reset_vector, &bytes)).unwrap();
            }
            Some(DifftestRef::FFI(_)) => {
                difftestffi_init(&state.regfile, bytes, reset_vector);
            }
            None => {
                state_ref = state.clone();
            }
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

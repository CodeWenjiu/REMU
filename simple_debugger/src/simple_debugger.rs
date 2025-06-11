use crate::cmd_parser::{Cmds, Server};
use cfg_if::cfg_if;
use logger::Logger;
use option_parser::OptionParser;
use remu_macro::log_err;
use simulator::Simulator;
use state::{model::StageModel, States};

use remu_utils::{DifftestRef, ItraceConfigtionalWrapper, ProcessError};

cfg_if! {
    if #[cfg(feature = "ITRACE")] {
    }
}

pub struct SimpleDebugger {
    server: Server,

    pub conditional: ItraceConfigtionalWrapper,

    pub state: States,
    pub state_ref: States,

    pub simulator: Simulator,
}

impl SimpleDebugger {
    pub fn new(cli_result: OptionParser) -> Result<Self, ()> {
        let conditional = ItraceConfigtionalWrapper::new(cli_result.cli.platform.isa);

        if let Some(difftest_ref) = cli_result.cli.differtest {
            Logger::function(
                &format!("differtest \"{}\"", difftest_ref).to_string(),
                true.into(),
            );
        } else {
            Logger::function("differtest", false.into());
        }

        let rl_history_length = cli_result
            .cfg
            .debug_config
            .rl_history_size;

        let (state, state_ref) = Self::state_init(&cli_result);

        let mut simulator = log_err!(Simulator::new(
            &cli_result,
            state.clone(),
            state_ref.clone(),
            conditional.clone()
        ))?;

        log_err!(simulator.load_memory(&cli_result))?;

        Ok(Self {
            server: Server::new(cli_result.cli.platform.simulator, rl_history_length)
                .expect("Unable to create server"),

            conditional,

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

        (state, state_ref)
    }

    pub fn mainloop(mut self, batch: bool) -> Result<(), ()> {

        if batch {
            match self.execute(Cmds::Continue) {
                Err(ProcessError::Recoverable) => return Ok(()),
                Err(ProcessError::GracefulExit) => return Ok(()),
                Err(ProcessError::Fatal) => return Err(()),
                _ => (),
            }
        }

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

            let cmds = handle_result!(self.server.get_parse());
            for cmd in cmds {
                handle_result!(self.execute(cmd));
            }
        }
    }
}

use crate::cmd_parser::{CmdParser, Cmds, Server};
use cfg_if::cfg_if;
use clap::Parser;
use logger::Logger;
use option_parser::OptionParser;
use remu_macro::{log_err, log_info};
use simulator::Simulator;
use state::{model::StageModel, States};

use remu_utils::{DifftestPipeline, DifftestRef, EmuSimulators, ItraceConfigtionalWrapper, ProcessError, ProcessResult, Simulators};

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

#[derive(pest_derive::Parser)]
#[grammar = "input_parser.pest"]
pub struct InputParser;

fn term_parse(pairs: pest::iterators::Pairs<Rule>) -> Vec<String> {
    pairs
        .into_iter()
        .map(|pair| 
            match pair.as_rule() {
                Rule::expr | Rule::cmd => pair.as_str().to_string(),
                _ => unreachable!()
            }
        )
        .collect()
}

fn input_parse(pairs: pest::iterators::Pairs<Rule>) -> Vec<Vec<String>> {
    pairs
        .into_iter()
        .map(|pair| 
            match pair.as_rule() {
                Rule::term => term_parse(pair.into_inner()),
                _ => unreachable!()
            }
        ).collect()
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

        let (state, state_ref) = Self::state_init(&cli_result, conditional.clone());

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

    fn state_init(cli_result: &OptionParser, conditional: ItraceConfigtionalWrapper) -> (States, States) {
        let isa = cli_result.cli.platform.isa;

        let reset_vector = cli_result.cfg.platform_config.reset_vector;

        let pipe_state = match cli_result.cli.platform.simulator {
            Simulators::NZEA(_) | Simulators::EMU(EmuSimulators::PL) => {
                StageModel::with_branchpredict(conditional.clone())
            }

            _ => {
                StageModel::default(conditional.clone())
            }
        };

        let mut state = States::new(isa, reset_vector, pipe_state).unwrap();
        let mut state_ref = state.clone();

        match cli_result.cli.differtest {
            Some(DifftestRef::SingleCycle(_)) => {
                state_ref = States::new(isa, reset_vector, StageModel::default(conditional)).unwrap();
            }

            Some(DifftestRef::Pipeline(platform)) => {
                match platform {
                    DifftestPipeline::EMU => {
                        state_ref = States::new(isa, reset_vector, StageModel::with_branchpredict(conditional)).unwrap();
                    }
                }
            }

            _ => {},
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

            match cli_result.cli.differtest {
                Some(DifftestRef::Pipeline(_)) | Some(DifftestRef::SingleCycle(_)) => {
                    log_err!(state_ref.mmu.add_region(
                        region.base,
                        region.size,
                        &region.name,
                        region.flag.clone(),
                        region.mmtype
                    ))
                    .unwrap();
                }

                _ => {},
            }
        }

        (state, state_ref)
    }

    fn get_parse(&mut self, lines: String) -> ProcessResult<Vec<Cmds>> {
        use pest::Parser;
        let pairs = log_err!(InputParser::parse(Rule::cmd_full, &lines), ProcessError::Recoverable)?;
        let lines = input_parse(pairs);

        let result: Vec<Cmds> = lines
            .into_iter()
            .map(|mut v| {
                v.insert(0, "".to_owned());
                v
            })
            .map(|line| {
                match CmdParser::try_parse_from(line) {
                    Ok(cmd) => Ok(cmd.command),
                    Err(e) if (e.kind() == clap::error::ErrorKind::DisplayHelp || e.kind() == clap::error::ErrorKind::DisplayVersion) => {
                        let _ = e.print();
                        Err(ProcessError::Recoverable)
                    }
                    Err(e) => {
                        let _ = e.print();
                        Err(ProcessError::Recoverable)
                    }
                }
            })
            .collect::<Result<Vec<Cmds>, ProcessError>>()?;

        return Ok(result);
    }

    pub fn mainloop(mut self, pre_exec: Option<String>) -> Result<(), ()> {
        macro_rules! handle_result {
            ($result:expr) => {
                match $result {
                    Err(ProcessError::Recoverable) => return Ok(()),
                    Err(ProcessError::GracefulExit) => return Ok(()),
                    Err(ProcessError::Fatal) => return Err(()),
                    Ok(value) => value,
                }
            };
        }

        if let Some(exec) = pre_exec {
            log_info!(format!("Executing pre-command {}", exec));
            let cmds = handle_result!(self.get_parse(exec));
            let combined_result = cmds.into_iter()
                .fold(Ok(()), |acc_result, cmd| {
                    acc_result.and(self.execute(cmd))
                });
            handle_result!(combined_result);
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

            let lines = handle_result!(self.server.readline());
            let cmds = handle_result!(self.get_parse(lines));
            for cmd in cmds {
                handle_result!(self.execute(cmd));
            }
        }
    }
}

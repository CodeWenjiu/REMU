use clap::Parser;
use logger::Logger;
use remu_macro::log_err;
use rustyline::{error::ReadlineError, highlight::MatchingBracketHighlighter, hint::HistoryHinter, history::{FileHistory, History}, validate::MatchingBracketValidator, CompletionType, Config, EditMode, Editor};

use remu_utils::{ProcessError, ProcessResult, Simulators};
use crate::cmd_parser::get_cmd_tree;

use super::{CmdCompleter, CmdParser, MyHelper};

pub struct Server {
    prompt: String,
    
    rl: Editor<MyHelper, FileHistory>,

    rl_history_length: usize,
}

#[derive(pest_derive::Parser)]
#[grammar = "cmd_parser/input_parser.pest"]
pub struct InputParser;

fn input_parse(pairs: pest::iterators::Pairs<Rule>) -> Vec<String> {
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

impl Server {
    pub fn new(sim: Simulators, rl_history_length: usize) -> Result<Self, ()> {
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build();
        
        let h = MyHelper {
            completer: CmdCompleter::new( get_cmd_tree()),
            highlighter: MatchingBracketHighlighter::new(),
            hinter: HistoryHinter::new(),
            colored_prompt: "".to_owned(),
            validator: MatchingBracketValidator::new(),
        };
        
        let mut rl = Editor::with_config(config).map_err(|e| eprintln!("{}", e))?;
        rl.set_helper(Some(h));
        if rl.load_history("./target/.rlhistory").is_err() {
            Logger::show("[readline] No previous history.", Logger::INFO);
        }

        let p = Logger::format(&("(".to_string() + sim.into() + ") -> "), Logger::IMPORTANT);

        rl.helper_mut().expect("No helper").colored_prompt = p.clone();

        Ok(
            Self {
                prompt: p,
                rl,
                rl_history_length,
            }
        )
    }

    pub fn get_parse(&mut self) -> ProcessResult<CmdParser> {
        loop {
            let line = self.readline()?;

            use pest::Parser;
            let pairs = log_err!(InputParser::parse(Rule::cmd_full, &line), ProcessError::Recoverable)?;
            let mut line = input_parse(pairs);

            if line.is_empty() {
                continue;
            }

            line.insert(0, "".to_owned());

            let cmd = CmdParser::try_parse_from(line);

            match cmd {
                Ok(cmd) => return Ok(cmd),
                Err(e) if (e.kind() == clap::error::ErrorKind::DisplayHelp || e.kind() == clap::error::ErrorKind::DisplayVersion) => {
                    let _ = e.print();
                    continue;
                }
                Err(e) => {
                    let _ = e.print();
                    continue;
                }
            }
        }
    }

    fn readline(&mut self) -> ProcessResult<String> {
        let readline = self.rl.readline(&self.prompt);

        match readline {
            Ok(mut line) => {
                if let Err(e) = self.rl.add_history_entry(line.as_str()) {
                    eprintln!("{}", e);
                    return Err(ProcessError::Fatal);
                }

                if line.is_empty() {
                    line = self.rl.history()
                        .iter()
                        .last()
                        .map_or("".to_owned(), |v| v.to_string());
                }

                Ok(line)
            }
            Err(ReadlineError::Interrupted) => {
                Ok("".to_string())
            }
            Err(ReadlineError::Eof) => {
                Logger::show("Quiting...", Logger::INFO);
                Err(ProcessError::GracefulExit)
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                Err(ProcessError::Fatal)
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // remove previous history until the length is satisfied
        let history_len = self.rl.history().len();
        if history_len > self.rl_history_length as usize {
            // Get a copy of the current history
            let history: Vec<String> = self.rl.history().iter().map(|entry| entry.to_string()).collect();
            
            // Clear the entire history
            if let Err(e) = self.rl.clear_history() {
                eprintln!("Error clearing history: {}", e);
            } else {
                // Re-add only the most recent entries
                for entry in history.iter().skip(history_len - self.rl_history_length as usize) {
                    let _ = self.rl.add_history_entry(entry);
                }
            }
        }

        self.rl.save_history("./target/.rlhistory").unwrap();
    }
}

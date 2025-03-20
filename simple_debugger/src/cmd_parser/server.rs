use clap::Parser;
use logger::Logger;
use rustyline::{error::ReadlineError, highlight::MatchingBracketHighlighter, hint::HistoryHinter, history::{FileHistory, History}, validate::MatchingBracketValidator, Cmd, CompletionType, Config, EditMode, Editor, KeyEvent};

use crate::cmd_parser::get_cmd_tree;

use super::{CmdCompleter, CmdParser, MyHelper};

pub struct Server {
    prompt: String,
    
    rl: Editor<MyHelper, FileHistory>,

    rl_history_length: u32,
}

pub enum ProcessResult<T> {
    Continue(T),
    Halt,
    Error,
}

impl Server {
    pub fn new(name: &str, rl_history_length: u32) -> Result<Self, ()> {
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
        rl.bind_sequence(KeyEvent::alt('n'), Cmd::HistorySearchForward);
        rl.bind_sequence(KeyEvent::alt('p'), Cmd::HistorySearchBackward);
        if rl.load_history("./target/.rlhistory").is_err() {
            Logger::show("[readline] No previous history.", Logger::INFO);
        }

        let p = Logger::format(&("(".to_string() + name + ") -> "), Logger::IMPORTANT);

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
            let line = self.readline();

            let line = match line {
                ProcessResult::Halt => return ProcessResult::Halt,
                ProcessResult::Error => return ProcessResult::Error,
                ProcessResult::Continue(line) => line,
            };

            let mut line = line.trim().split_whitespace().collect::<Vec<&str>>();
            if line.is_empty() {
                continue;
            }

            line.insert(0, "");

            let cmd = CmdParser::try_parse_from(line);

            match cmd {
                Ok(cmd) => return ProcessResult::Continue(cmd),
                Err(e) if (e.kind() == clap::error::ErrorKind::DisplayHelp || e.kind() == clap::error::ErrorKind::DisplayVersion) => {
                    let _ = e.print();
                    continue;
                }
                Err(_) => {
                    Logger::show("Invalid command", Logger::ERROR);
                    continue;
                }
            }
        }
    }

    fn readline(&mut self) -> ProcessResult<String> {
        let readline = self.rl.readline(&self.prompt);

        match readline {
            Ok(line) => {
                if let Err(e) = self.rl.add_history_entry(line.as_str()) {
                    eprintln!("{}", e);
                    return ProcessResult::Error;
                }

                ProcessResult::Continue(line)
            }
            Err(ReadlineError::Interrupted) => {
                ProcessResult::Continue("".to_string())
            }
            Err(ReadlineError::Eof) => {
                Logger::show("Quiting...", Logger::INFO);
                ProcessResult::Halt
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                ProcessResult::Error
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

use clap::CommandFactory;
use logger::Logger;
use rustyline::{error::ReadlineError, highlight::MatchingBracketHighlighter, hint::HistoryHinter, history::FileHistory, validate::MatchingBracketValidator, Cmd, CompletionType, Config, EditMode, Editor, KeyEvent};

use super::{CmdCompleter, CmdParser, MyHelper};

pub struct Server {
    prompt: String,
    
    rl: Editor<MyHelper, FileHistory>,
}

pub enum ProcessResult {
    Continue(String),
    Halt,
    Error,
}

impl Server {
    pub fn new(name: &str) -> Result<Self, ()> {
        let cmds_vec: Vec<String> = CmdParser::command()
            .get_subcommands()
            .map(|subcmd| subcmd.get_name().to_string())
            .collect();

        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build();
        
        let h = MyHelper {
            completer: CmdCompleter::new(cmds_vec),
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
            }
        )
    }

    pub fn readline(&mut self) -> ProcessResult {
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
                Logger::show("Interrupt", Logger::INFO);
                ProcessResult::Halt
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
        self.rl.save_history("./target/.rlhistory").unwrap();
    }
}

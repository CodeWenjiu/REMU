use owo_colors::OwoColorize;
use rustyline::{error::ReadlineError, highlight::MatchingBracketHighlighter, hint::HistoryHinter, history::FileHistory, validate::MatchingBracketValidator, Cmd, CompletionType, Config, EditMode, Editor, KeyEvent};

use super::{CmdCompleter, MyHelper};

pub struct Server {
    name: String,
    rl: Editor<MyHelper, FileHistory>
}

pub enum ProcessResult {
    Continue(String),
    Halt,
    Error,
}

impl Server {
    pub fn new(name: &str) -> Result<Self, ()> {
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build();
        
        let h = MyHelper {
            completer: CmdCompleter::new(),
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
            println!("No previous history.");
        }

        Ok(
            Self {
                name: name.to_string(),
                rl
            }
        )
    }

    pub fn readline(&mut self) -> ProcessResult {
        let p = format!("✨{} -> ", self.name);
        self.rl.helper_mut().expect("No helper").colored_prompt = p.purple().to_string();
        let readline = self.rl.readline(&p);

        match readline {
            Ok(line) => {
                if let Err(e) = self.rl.add_history_entry(line.as_str()) {
                    eprintln!("{}", e);
                    return ProcessResult::Error;
                }

                ProcessResult::Continue(line)
            }
            Err(ReadlineError::Interrupted) => {
                println!("Interrupted");
                ProcessResult::Halt
            }
            Err(ReadlineError::Eof) => {
                println!("Quiting...");
                ProcessResult::Halt
            }
            Err(err) => {
                println!("Error: {err:?}");
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

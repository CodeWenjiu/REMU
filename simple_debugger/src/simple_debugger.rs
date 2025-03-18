use option_parser::OptionParser;
use owo_colors::OwoColorize;
use rustyline::{error::ReadlineError, highlight::MatchingBracketHighlighter, hint::HistoryHinter, validate::MatchingBracketValidator, Cmd, CompletionType, Config, EditMode, Editor, KeyEvent};

use crate::cmd_parser::{CmdCompleter, MyHelper};

pub struct SimpleDebugger {
    pub name: String,
}

impl SimpleDebugger {
    pub fn new(cli_result: OptionParser) -> Self {
        let (_isa, name) = cli_result.cli.platform.split_once('-').unwrap();

        Self {
            name: name.to_string(),
        }
    }

    pub fn mainloop(self) -> Result<(), ()> {
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
        if rl.load_history("history.txt").is_err() {
            println!("No previous history.");
        }

        loop {
            let p = format!("✨{} -> ", self.name);
            rl.helper_mut().expect("No helper").colored_prompt = p.purple().to_string();
            let readline = rl.readline(&p);
            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str()).map_err(|e| eprintln!("{}", e))?;
                    println!("Line: {line}");
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Interrupted");
                    return Ok(());
                }
                Err(ReadlineError::Eof) => {
                    println!("Encountered Eof");
                    return Ok(());
                }
                Err(err) => {
                    println!("Error: {err:?}");
                    return Err(());
                }
            }
        }
    }
}

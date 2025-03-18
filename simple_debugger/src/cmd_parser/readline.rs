use std::borrow::Cow::{self, Borrowed, Owned};

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter, MatchingBracketHighlighter};
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::Context;
use rustyline::{Completer, Helper, Hinter, Validator};

pub struct CmdCompleter {
    commands: Vec<String>,
}

impl CmdCompleter {
    pub fn new() -> CmdCompleter {
        CmdCompleter {
            commands: vec!["ls".to_string(), "echo".to_string(), "cat".to_string()],
        }
    }

    fn complete_path(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let mut completions = vec![];
        let start = line[..pos].rfind(' ').map_or(0, |i| i + 1);
        let word = &line[start..pos];
        for cmd in &self.commands {
            if cmd.starts_with(word) {
                completions.push(Pair {
                    display: cmd.clone(),
                    replacement: cmd.clone(),
                });
            }
        }
        Ok((start, completions))
    }
}

impl Completer for CmdCompleter {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.complete_path(line, pos)
    }
}

#[derive(Helper, Completer, Hinter, Validator)]
pub struct MyHelper {
    #[rustyline(Completer)]
    pub completer: CmdCompleter,
    pub highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    pub validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    pub hinter: HistoryHinter,
    pub colored_prompt: String,
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        self.highlighter.highlight_char(line, pos, kind)
    }
}
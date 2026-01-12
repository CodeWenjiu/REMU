use reedline::{Completer, Span, Suggestion};

#[derive(Clone)]
pub struct RemuCompleter {
    commands: Vec<String>,
}

impl RemuCompleter {
    pub fn new(commands: Vec<String>) -> Self {
        Self { commands }
    }
}

impl Completer for RemuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        self.commands
            .iter()
            .filter(|cmd| cmd.starts_with(line))
            .map(|cmd| Suggestion {
                value: cmd.clone(),
                description: None,
                style: None,
                extra: None,
                span: Span::new(0, pos),
                append_whitespace: true,
                match_indices: Some((0..line.len()).collect()),
            })
            .collect()
    }
}

use nu_ansi_term::{Color, Style};
use petgraph::graph::{Graph, NodeIndex};
use reedline::{Highlighter, StyledText};

/// A highlighter that builds its keyword list from the petgraph command tree.
/// Keeps highlighting in sync with `remu_core::get_command_graph()`.
pub struct RemuHighlighter {
    graph: Graph<String, ()>,
    root: NodeIndex,
}

impl RemuHighlighter {
    /// Build a `RemuHighlighter` from a caller-provided command graph.
    pub fn new(graph: Graph<String, ()>, root: NodeIndex) -> Self {
        Self { graph, root }
    }
}

impl Highlighter for RemuHighlighter {
    fn highlight(&self, line: &str, _: usize) -> StyledText {
        let mut styled = StyledText::new();
        let mut idx = 0;
        let mut current = self.root;
        let mut path_ok = true;

        while idx < line.len() {
            let ws_end = line[idx..]
                .find(|c: char| !c.is_whitespace())
                .map(|o| idx + o)
                .unwrap_or(line.len());
            if ws_end > idx {
                styled.push((Style::new(), line[idx..ws_end].to_string()));
                idx = ws_end;
                if idx >= line.len() {
                    break;
                }
            }
            let token_end = line[idx..]
                .find(char::is_whitespace)
                .map(|o| idx + o)
                .unwrap_or(line.len());
            let token = &line[idx..token_end];

            let (style, next_current) = if path_ok {
                if let Some(nxt) = self
                    .graph
                    .neighbors(current)
                    .find(|n| self.graph[*n] == token)
                {
                    (Style::new().fg(Color::Green), Some(nxt))
                } else {
                    path_ok = false;
                    (Style::new().fg(Color::Red), None)
                }
            } else {
                (Style::new().fg(Color::Red), None)
            };

            styled.push((style, token.to_string()));
            if let Some(nxt) = next_current {
                current = nxt;
            }

            idx = token_end;
        }

        styled
    }
}

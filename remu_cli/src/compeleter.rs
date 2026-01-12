use petgraph::graph::{Graph, NodeIndex};
use reedline::{Completer, Span, Suggestion};

#[derive(Clone)]
pub struct RemuCompleter {
    graph: Graph<String, ()>,
    root: NodeIndex,
}

impl RemuCompleter {
    pub fn new(graph: Graph<String, ()>, root: NodeIndex) -> Self {
        Self { graph, root }
    }
}

impl Completer for RemuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        // Split by whitespace, preserving an empty tail when the cursor is after a space.
        let mut parts: Vec<&str> = line.trim_start().split_whitespace().collect();
        if line.ends_with(char::is_whitespace) {
            parts.push("");
        }
        if parts.is_empty() {
            parts.push("");
        }

        // Navigate the graph using all but the last token.
        let mut current = self.root;
        for token in parts.iter().take(parts.len().saturating_sub(1)) {
            if let Some(next) = self
                .graph
                .neighbors(current)
                .find(|n| self.graph[*n] == *token)
            {
                current = next;
            } else {
                return vec![];
            }
        }

        let needle = parts.last().copied().unwrap_or_default();
        let start = pos.saturating_sub(needle.len());

        self.graph
            .neighbors(current)
            .filter_map(|child| {
                let name = &self.graph[child];
                if name.starts_with(needle) {
                    Some(Suggestion {
                        value: name.clone(),
                        description: None,
                        style: None,
                        extra: None,
                        span: Span::new(start, pos),
                        append_whitespace: true,
                        match_indices: Some((0..needle.len()).collect()),
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

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

    /// If `pos` is inside a `{ ... }` block, return (inner_text, inner_cursor, base_offset).
    ///
    /// - If the closing `}` exists, the scope ends at that `}`.
    /// - If the closing `}` does not exist yet (while editing), the scope ends at end-of-line.
    fn current_brace_scope(line: &str, pos: usize) -> Option<(&str, usize, usize)> {
        if pos > line.len() {
            return None;
        }

        // Find the nearest unmatched '{' to the left of cursor.
        let mut stack: Vec<usize> = Vec::new();
        for (i, b) in line.as_bytes().iter().enumerate().take(pos) {
            match *b {
                b'{' => stack.push(i),
                b'}' => {
                    let _ = stack.pop();
                }
                _ => {}
            }
        }

        let open = *stack.last()?;
        // Cursor must be after '{' to be considered "inside" the block.
        // Allow completing immediately after '{' (including "{|" and "{ |").
        if pos <= open {
            return None;
        }

        // Find the matching '}' to the right of cursor.
        // If there is no closing brace yet, treat the scope as running until end-of-line.
        let mut depth: isize = 0;
        let mut close: Option<usize> = None;
        for (i_rel, b) in line.as_bytes()[pos..].iter().enumerate() {
            match *b {
                b'{' => depth += 1,
                b'}' => {
                    if depth == 0 {
                        close = Some(pos + i_rel);
                        break;
                    } else {
                        depth -= 1;
                    }
                }
                _ => {}
            }
        }
        let close = close.unwrap_or(line.len());

        // Inner content is between braces.
        let inner_start = open + 1;
        let inner_end = close;
        if inner_start > inner_end || inner_end > line.len() {
            return None;
        }

        let inner = &line[inner_start..inner_end];
        let inner_pos = pos.saturating_sub(inner_start);
        Some((inner, inner_pos, inner_start))
    }

    fn complete_within_graph(&self, line: &str, pos: usize) -> Vec<Suggestion> {
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

impl Completer for RemuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        // If the cursor is inside a complete `{ ... }` block, complete within that scope.
        if let Some((inner, inner_pos, base)) = Self::current_brace_scope(line, pos) {
            let mut out = self.complete_within_graph(inner, inner_pos);

            // Remap spans from inner coordinates back to the full line coordinates.
            for s in out.iter_mut() {
                s.span = Span::new(base + s.span.start, base + s.span.end);
            }

            return out;
        }

        // Otherwise, fall back to normal single-command completion on the whole line.
        self.complete_within_graph(line, pos)
    }
}

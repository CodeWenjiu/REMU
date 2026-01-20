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

    /// Attempt to offer structural completions when the cursor is *not* inside `{ ... }`.
    ///
    /// Goals:
    /// - After a completed `{ ... }` block, suggest `and {}` / `or {}` and place cursor inside `{}`.
    /// - When input is blank / only whitespace, *do not* override normal command completion.
    ///   (We intentionally do not return `{}` here; the caller may decide to merge `{}` alongside
    ///   normal command suggestions if desired.)
    fn complete_structural_outside_braces(&self, line: &str, pos: usize) -> Vec<Suggestion> {
        if pos > line.len() {
            return vec![];
        }

        // Boundary condition: empty/whitespace input should still offer normal command completion,
        // so don't return structural suggestions from this helper.
        if line.trim().is_empty() {
            return vec![];
        }

        // Only provide structural suggestions when cursor is at end-of-line.
        // This matches the "after a {} statement" wording and avoids surprising mid-line edits.
        if pos != line.len() {
            return vec![];
        }

        let bytes = line.as_bytes();

        // Helper: skip trailing whitespace and return index of last non-ws byte (inclusive).
        let mut i = bytes.len().saturating_sub(1);
        while i > 0 && bytes[i].is_ascii_whitespace() {
            i = i.saturating_sub(1);
        }
        if bytes[i].is_ascii_whitespace() {
            return vec![];
        }

        // We only offer `and {}` / `or {}` if the line ends with a full brace block (`...}`).
        if bytes[i] != b'}' {
            return vec![];
        }

        // Walk backwards to find the matching '{' for this closing brace.
        let mut depth: isize = 0;
        let mut j = i;
        loop {
            match bytes[j] {
                b'}' => depth += 1,
                b'{' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }

            if j == 0 {
                return vec![];
            }
            j -= 1;
        }

        // `j` is the matching '{' for the last '}'.
        // Ensure there is no extra non-ws content after the closing brace (we already trimmed trailing ws),
        // so this truly ends with a complete block.
        // Now suggest ` and {}` / ` or {}`. We include a leading space to keep formatting natural.
        // Span replaces an empty range at the cursor (insertion).
        let insert_span = Span::new(pos, pos);

        vec![
            Suggestion {
                value: " and {".to_string(),
                description: Some("structural".to_string()),
                style: None,
                extra: None,
                span: insert_span,
                append_whitespace: false,
                match_indices: None,
            },
            Suggestion {
                value: " or {".to_string(),
                description: Some("structural".to_string()),
                style: None,
                extra: None,
                span: insert_span,
                append_whitespace: false,
                match_indices: None,
            },
        ]
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

        // Outside of braces, try structural completions first (and/or + {}).
        let structural = self.complete_structural_outside_braces(line, pos);
        if !structural.is_empty() {
            return structural;
        }

        // Otherwise, fall back to normal single-command completion on the whole line.
        self.complete_within_graph(line, pos)
    }
}

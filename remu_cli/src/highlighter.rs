use petgraph::{
    graph::{Graph, NodeIndex},
    visit::Dfs,
};
use reedline::{ExampleHighlighter, Highlighter, StyledText};

/// A highlighter that builds its keyword list from the petgraph command tree.
/// Keeps highlighting in sync with `remu_core::get_command_graph()`.
pub struct RemuHighlighter {
    inner: ExampleHighlighter,
}

impl RemuHighlighter {
    /// Build a `RemuHighlighter` from a caller-provided command graph.
    pub fn new(graph: Graph<String, ()>, root: NodeIndex) -> Self {
        // Flatten all command names (excluding the synthetic root).
        let mut dfs = Dfs::new(&graph, root);
        let mut words = Vec::new();
        while let Some(nx) = dfs.next(&graph) {
            if nx != root {
                words.push(graph[nx].clone());
            }
        }

        Self {
            inner: ExampleHighlighter::new(words),
        }
    }
}

impl Highlighter for RemuHighlighter {
    fn highlight(&self, line: &str, pos: usize) -> StyledText {
        self.inner.highlight(line, pos)
    }
}

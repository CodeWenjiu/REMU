use nu_ansi_term::{Color, Style};
use petgraph::graph::{Graph, NodeIndex};
use reedline::{Highlighter, StyledText};
use remu_debugger::{ExprParser, Rule};
use remu_harness::CommandParser;
use std::collections::BTreeMap;

/// A highlighter using pest parser for validation.
/// - Keywords (and, or, {, }) are yellow when structurally valid
/// - Commands are green when valid in the graph
/// - Everything invalid is red
pub struct RemuHighlighter {
    graph: Graph<String, ()>,
    root: NodeIndex,
}

impl RemuHighlighter {
    pub fn new(graph: Graph<String, ()>, root: NodeIndex) -> Self {
        Self { graph, root }
    }

    /// Check if a token exists as a valid command from current node
    fn is_valid_command(&self, current: NodeIndex, token: &str) -> Option<NodeIndex> {
        self.graph
            .neighbors(current)
            .find(|n| self.graph[*n] == token)
    }

    /// Highlight line by parsing with pest and mapping character styles
    fn highlight_parsed(&self, line: &str) -> Option<StyledText> {
        let pairs = <ExprParser as pest::Parser<Rule>>::parse(Rule::expr, line).ok()?;

        // Create a character-level style map: byte position -> Style
        let mut style_map: BTreeMap<usize, Style> = BTreeMap::new();

        // Initialize all positions with default style
        for i in 0..line.len() {
            style_map.insert(i, Style::new());
        }

        for pair in pairs {
            self.collect_styles(pair, line, &mut style_map, self.root);
        }

        // Build StyledText from the style map
        self.build_styled_text_from_map(line, &style_map)
    }

    /// Recursively collect styles for each character position from parse tree
    fn collect_styles(
        &self,
        pair: pest::iterators::Pair<Rule>,
        full_line: &str,
        style_map: &mut BTreeMap<usize, Style>,
        mut current: NodeIndex,
    ) -> NodeIndex {
        let span = pair.as_span();
        let start = span.start();
        let end = span.end();

        match pair.as_rule() {
            Rule::expr => {
                for inner in pair.into_inner() {
                    current = self.collect_styles(inner, full_line, style_map, current);
                }
            }
            Rule::block => {
                for inner in pair.into_inner() {
                    current = self.collect_styles(inner, full_line, style_map, current);
                }
            }
            Rule::brace_block => {
                let mut inner_iter = pair.into_inner();

                // Find opening brace and mark it yellow
                if let Some(brace_idx) = full_line[start..end].find('{') {
                    let abs_brace_idx = start + brace_idx;
                    style_map.insert(abs_brace_idx, Style::new().fg(Color::Cyan));
                }

                // Process inner content
                let inner_current = self.root;
                if let Some(inner_pair) = inner_iter.find(|p| p.as_rule() == Rule::inner) {
                    let _ = self.collect_styles(inner_pair, full_line, style_map, inner_current);
                }

                // Find closing brace and mark it yellow
                if let Some(brace_idx) = full_line[start..end].rfind('}') {
                    let abs_brace_idx = start + brace_idx;
                    style_map.insert(abs_brace_idx, Style::new().fg(Color::Cyan));
                }

                current = self.root;
            }
            Rule::command => {
                let text = &full_line[start..end];
                current = self.style_command_segment(text, start, style_map, current);
            }
            Rule::inner => {
                // Inner content of do block
                let text = &full_line[start..end];
                current = self.style_command_segment(text, start, style_map, current);
            }
            Rule::and | Rule::or => {
                // Style operator keyword as yellow
                for i in start..end {
                    style_map.insert(i, Style::new().fg(Color::Cyan));
                }
                current = self.root;
            }
            Rule::WS => {
                // Whitespace gets default style (already initialized)
            }
            Rule::EOI => {
                // End of input
            }
            _ => {
                // Other rules - keep default style
            }
        }

        current
    }

    /// Style the tokens in a command segment
    fn style_command_segment(
        &self,
        text: &str,
        abs_start: usize,
        style_map: &mut BTreeMap<usize, Style>,
        mut current: NodeIndex,
    ) -> NodeIndex {
        // Highest priority: if clap accepts this whole command segment, render it all green.
        // NOTE: CommandParser expects argv[0] to be the binary name.
        let mut argv: Vec<String> = Vec::new();
        argv.push(env!("CARGO_PKG_NAME").to_string());
        argv.extend(text.split_whitespace().map(|s| s.to_string()));

        let clap_parse_result = <CommandParser as clap::Parser>::try_parse_from(argv);
        let is_valid = match &clap_parse_result {
            Ok(_) => true,
            Err(e) => matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ),
        };

        if is_valid {
            for i in abs_start..(abs_start + text.len()) {
                style_map.insert(i, Style::new().fg(Color::Green));
            }
            return current;
        }

        let tokens: Vec<&str> = text.split_whitespace().collect();

        if tokens.is_empty() {
            return current;
        }

        let mut text_pos = 0;

        for token in tokens.iter() {
            // Find the token in the text starting from text_pos
            if let Some(token_start_in_text) = text[text_pos..].find(token) {
                let token_start_abs = abs_start + text_pos + token_start_in_text;
                let token_end_abs = token_start_abs + token.len();

                // Check if this is a valid command
                if let Some(next) = self.is_valid_command(current, token) {
                    // Token is a valid command in the graph, but the full command segment is not
                    // clap-valid (otherwise we'd have returned early above). Mark as yellow.
                    for i in token_start_abs..token_end_abs {
                        style_map.insert(i, Style::new().fg(Color::Yellow));
                    }
                    current = next;
                } else {
                    // Invalid command - color red
                    for i in token_start_abs..token_end_abs {
                        style_map.insert(i, Style::new().fg(Color::Red));
                    }
                }

                text_pos = token_start_in_text + token.len();
            } else {
                // Shouldn't happen, but color as red to be safe
                for i in 0..token.len() {
                    style_map.insert(abs_start + text_pos + i, Style::new().fg(Color::Red));
                }
            }
        }

        current
    }

    /// Build StyledText by grouping consecutive characters with same style
    fn build_styled_text_from_map(
        &self,
        line: &str,
        style_map: &BTreeMap<usize, Style>,
    ) -> Option<StyledText> {
        let mut out = StyledText::new();
        let mut current_style = Style::new();
        let mut current_text = String::new();

        for (pos, ch) in line.chars().enumerate() {
            let style = style_map.get(&pos).copied().unwrap_or_else(Style::new);

            if style == current_style {
                current_text.push(ch);
            } else {
                // Style changed, push accumulated text
                if !current_text.is_empty() {
                    out.push((current_style, current_text.clone()));
                    current_text.clear();
                }
                current_style = style;
                current_text.push(ch);
            }
        }

        // Push any remaining text
        if !current_text.is_empty() {
            out.push((current_style, current_text));
        }

        Some(out)
    }
}

impl Highlighter for RemuHighlighter {
    fn highlight(&self, line: &str, _: usize) -> StyledText {
        // Try pest-based highlighting first
        if let Some(styled) = self.highlight_parsed(line) {
            return styled;
        }

        // Fallback for incomplete/invalid input
        self.highlight_fallback(line)
    }
}

impl RemuHighlighter {
    /// Fallback highlighter for incomplete input
    fn highlight_fallback(&self, line: &str) -> StyledText {
        let mut out = StyledText::new();

        // Tokenize preserving whitespace and braces
        let mut tokens: Vec<(String, bool)> = Vec::new(); // (segment, is_ws)
        let mut buf = String::new();
        let mut is_ws = line
            .chars()
            .next()
            .map(|c| c.is_whitespace())
            .unwrap_or(false);

        for ch in line.chars() {
            let ws = ch.is_whitespace();
            if ws != is_ws || ch == '{' || ch == '}' {
                if !buf.is_empty() {
                    tokens.push((buf.clone(), is_ws));
                    buf.clear();
                }
                if ch == '{' || ch == '}' {
                    tokens.push((ch.to_string(), false));
                    is_ws = true;
                    continue;
                }
                is_ws = ws;
            }
            buf.push(ch);
        }
        if !buf.is_empty() {
            tokens.push((buf, is_ws));
        }

        // State machine for fallback mode
        let mut current = self.root;
        let mut expect_block = true;
        let mut in_do_block = false;
        let mut pending_do = false;

        for (i, (seg, ws)) in tokens.iter().enumerate() {
            if *ws {
                out.push((Style::new(), seg.clone()));
                continue;
            }

            let next_token = tokens
                .iter()
                .skip(i + 1)
                .find_map(|(s, is_ws)| if !*is_ws { Some(s.as_str()) } else { None });

            let token = seg.as_str();
            let styled_seg;

            if token == "{" {
                // In the simplified syntax, a block starts directly with "{") when a block is expected.
                let valid = expect_block && !in_do_block && !pending_do;
                styled_seg = (
                    Style::new().fg(if valid { Color::Cyan } else { Color::Red }),
                    token.to_string(),
                );
                pending_do = false;
                in_do_block = valid;
                current = self.root;
                expect_block = false;
            } else if token == "}" {
                let valid = in_do_block;
                styled_seg = (
                    Style::new().fg(if valid { Color::Cyan } else { Color::Red }),
                    token.to_string(),
                );
                in_do_block = false;
                expect_block = false;
            } else if token == "and" || token == "or" {
                // After a complete block, an operator must be followed by another block ("{").
                let valid = !in_do_block && !expect_block && matches!(next_token, Some("{"));
                styled_seg = (
                    Style::new().fg(if valid { Color::Cyan } else { Color::Red }),
                    token.to_string(),
                );
                expect_block = true;
                current = self.root;
            } else {
                let is_valid = self.is_valid_command(current, token).is_some();
                styled_seg = (
                    Style::new().fg(if is_valid { Color::Green } else { Color::Red }),
                    token.to_string(),
                );
                if is_valid {
                    if let Some(next) = self.is_valid_command(current, token) {
                        current = next;
                    }
                }
            }

            out.push(styled_seg);
        }

        out
    }
}

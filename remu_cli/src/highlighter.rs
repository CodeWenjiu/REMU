use nu_ansi_term::{Color, Style};
use petgraph::graph::{Graph, NodeIndex};
use reedline::{Highlighter, StyledText};
use remu_simulator::SimulatorCommand;
use winnow::prelude::*;
use winnow::token::take_while;

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
}

impl Highlighter for RemuHighlighter {
    fn highlight(&self, line: &str, _: usize) -> StyledText {
        self.highlight_fallback(line)
    }
}

impl RemuHighlighter {
    /// Command expression highlighter (winnow-based, spans preserved)
    ///
    /// Grammar (equivalent intent to the old pest version):
    /// - expr  := block (WS* (and|or) WS* block)* WS*
    /// - block := command | brace_block
    /// - brace_block := "{" inner? "}"
    /// - command := words until a top-level (and|or) token is encountered
    ///
    /// Notes:
    /// - Inside `{ ... }`, we **do not** parse logical operators; inner content is treated as a command segment.
    /// - `and/or` only count as operators if they are standalone tokens (followed by WS or EOI),
    ///   and only at top-level (not inside braces).
    fn highlight_fallback(&self, line: &str) -> StyledText {
        let mut out = StyledText::new();

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum TokKind {
            Ws,
            LBrace,
            RBrace,
            Word,
        }

        #[derive(Debug, Clone)]
        struct Tok<'a> {
            kind: TokKind,
            text: &'a str,
            end: usize,
        }

        fn parse_one<'a>(input: &mut &'a str, offset: usize) -> winnow::Result<Tok<'a>> {
            // Whitespace
            if input.starts_with(char::is_whitespace) {
                let ws = take_while(1.., |c: char| c.is_whitespace()).parse_next(input)?;
                let len = ws.len();
                return Ok(Tok {
                    kind: TokKind::Ws,
                    text: ws,
                    end: offset + len,
                });
            }

            // Braces
            if input.starts_with('{') {
                "{".parse_next(input)?;
                return Ok(Tok {
                    kind: TokKind::LBrace,
                    text: "{",
                    end: offset + 1,
                });
            }
            if input.starts_with('}') {
                "}".parse_next(input)?;
                return Ok(Tok {
                    kind: TokKind::RBrace,
                    text: "}",
                    end: offset + 1,
                });
            }

            // Word: run until whitespace or brace
            let word = take_while(1.., |c: char| !c.is_whitespace() && c != '{' && c != '}')
                .parse_next(input)?;
            let len = word.len();
            Ok(Tok {
                kind: TokKind::Word,
                text: word,
                end: offset + len,
            })
        }

        fn tokenize(line: &str) -> Vec<Tok<'_>> {
            let mut toks = Vec::new();
            let mut rest = line;
            let mut offset = 0usize;

            while !rest.is_empty() {
                let before = rest.len();
                match parse_one(&mut rest, offset) {
                    Ok(tok) => {
                        offset = tok.end;
                        toks.push(tok);
                    }
                    Err(_) => {
                        // Shouldn't happen; make progress by consuming 1 byte.
                        let b = &rest[..1];
                        toks.push(Tok {
                            kind: TokKind::Word,
                            text: b,
                            end: offset + 1,
                        });
                        rest = &rest[1..];
                        offset += 1;
                    }
                }
                // Safety: ensure progress
                if rest.len() == before {
                    break;
                }
            }

            toks
        }

        fn is_ws_byte(b: u8) -> bool {
            matches!(b, b' ' | b'\t' | b'\n' | b'\r')
        }

        fn is_operator_token(word: &str, next_byte: Option<u8>) -> bool {
            match word {
                "and" => next_byte.map(is_ws_byte).unwrap_or(true),
                "or" => next_byte.map(is_ws_byte).unwrap_or(true),
                _ => false,
            }
        }

        let toks = tokenize(line);

        let mut current = self.root;
        let mut in_brace = false;

        // Start index (inclusive) in `toks` of the current command segment.
        let mut seg_start = 0usize;

        // Whether the current command segment is syntactically "expected" (used for brace validity).
        let mut expect_block = true;

        // Validate whether a token is a syntactic operator at top-level.
        // IMPORTANT: make this a plain function-style closure by passing `in_brace`,
        // so we don't borrow `in_brace` for the whole loop body.
        let is_top_level_op_at = |i: usize, in_brace: bool| -> bool {
            if in_brace {
                return false;
            }
            let t = &toks[i];
            if t.kind != TokKind::Word {
                return false;
            }
            // Determine next byte in the original line after this token's text.
            let next_byte = line.as_bytes().get(t.end).copied();
            is_operator_token(t.text, next_byte)
        };

        // Apply command coloring for a segment [seg_start, seg_end) (token indices).
        fn flush_segment(
            out: &mut StyledText,
            toks: &[Tok<'_>],
            seg_start: usize,
            seg_end: usize,
            highlighter: &RemuHighlighter,
            mut current: NodeIndex,
        ) -> NodeIndex {
            // Highest priority: if clap accepts this whole command segment, render it all green.
            // NOTE: CommandParser expects argv[0] to be the binary name.
            let mut argv: Vec<String> = Vec::new();
            argv.push(env!("CARGO_PKG_NAME").to_string());

            // Segment validity check should ignore whitespace and braces.
            for tok in toks[seg_start..seg_end].iter() {
                match tok.kind {
                    TokKind::Ws => {}
                    TokKind::LBrace | TokKind::RBrace => {}
                    TokKind::Word => {
                        argv.push(tok.text.to_string());
                    }
                }
            }

            let clap_parse_result = <SimulatorCommand as clap::Parser>::try_parse_from(argv);
            let clap_valid = match &clap_parse_result {
                Ok(_) => true,
                Err(e) => matches!(
                    e.kind(),
                    clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
                ),
            };

            if clap_valid {
                for tok in toks[seg_start..seg_end].iter() {
                    out.push((Style::new().fg(Color::Green), tok.text.to_string()));
                }
                return current;
            }

            for tok in toks[seg_start..seg_end].iter() {
                match tok.kind {
                    TokKind::Ws => {
                        out.push((Style::new(), tok.text.to_string()));
                    }
                    TokKind::LBrace | TokKind::RBrace => {
                        // Brace handling is outside; shouldn't occur here.
                        out.push((Style::new().fg(Color::Red), tok.text.to_string()));
                    }
                    TokKind::Word => {
                        let token = tok.text;

                        // Operator tokens should not appear inside a command segment.
                        if token == "and" || token == "or" {
                            out.push((Style::new().fg(Color::Red), token.to_string()));
                            current = highlighter.root;
                            continue;
                        }

                        // Command graph validation
                        let is_valid = highlighter.is_valid_command(current, token).is_some();
                        out.push((
                            Style::new().fg(if is_valid { Color::Yellow } else { Color::Red }),
                            token.to_string(),
                        ));

                        if is_valid {
                            if let Some(next) = highlighter.is_valid_command(current, token) {
                                current = next;
                            }
                        }
                    }
                }
            }

            current
        }

        // Iterate tokens, splitting into: command segments and operator / brace tokens.
        let mut i = 0usize;
        while i < toks.len() {
            let tok = &toks[i];

            match tok.kind {
                TokKind::Ws => {
                    // Whitespace is part of whichever segment we're currently building.
                    i += 1;
                }
                TokKind::LBrace => {
                    // Flush anything before brace as a command segment.
                    if i > seg_start {
                        let _ = flush_segment(&mut out, &toks, seg_start, i, self, current);
                    }

                    // LBrace validity: only allowed when expecting a block and not already in brace.
                    let valid = expect_block && !in_brace;
                    out.push((
                        Style::new().fg(if valid { Color::Cyan } else { Color::Red }),
                        "{".to_string(),
                    ));

                    in_brace = valid;
                    expect_block = false;

                    // Reset command graph on entering a block.
                    current = self.root;

                    // Next segment starts after brace.
                    seg_start = i + 1;
                    i += 1;
                }
                TokKind::RBrace => {
                    // Flush inner content as a command segment (brace-inner command).
                    if i > seg_start {
                        let _ = flush_segment(&mut out, &toks, seg_start, i, self, self.root);
                    }

                    let valid = in_brace;
                    out.push((
                        Style::new().fg(if valid { Color::Cyan } else { Color::Red }),
                        "}".to_string(),
                    ));

                    in_brace = false;
                    expect_block = false;

                    // Reset graph after completing a block.
                    current = self.root;

                    seg_start = i + 1;
                    i += 1;
                }
                TokKind::Word => {
                    if is_top_level_op_at(i, in_brace) {
                        // Flush preceding command segment.
                        if i > seg_start {
                            let _ = flush_segment(&mut out, &toks, seg_start, i, self, current);
                        }

                        // Operator validity: must be between blocks (i.e., just parsed a block/command),
                        // and we must now expect another block.
                        let valid = !in_brace && !expect_block;

                        out.push((
                            Style::new().fg(if valid { Color::Cyan } else { Color::Red }),
                            tok.text.to_string(),
                        ));

                        expect_block = true;
                        current = self.root;

                        seg_start = i + 1;
                        i += 1;
                    } else {
                        // Part of a command segment.
                        expect_block = false;
                        i += 1;
                    }
                }
            }
        }

        // Flush trailing segment.
        if seg_start < toks.len() {
            let _ = flush_segment(&mut out, &toks, seg_start, toks.len(), self, current);
        }

        out
    }
}

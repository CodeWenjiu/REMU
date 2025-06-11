use std::borrow::Cow::{self, Borrowed, Owned};
use owo_colors::OwoColorize;
use pest::Parser;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter, MatchingBracketHighlighter};
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::Context;
use rustyline::{Completer, Helper, Hinter, Validator};

use crate::cmd_parser::server::InputParser;
use crate::cmd_parser::server::Rule;

pub struct CmdCompleter {
    cmds_tree: Graph<String, ()>,
}

impl CmdCompleter {
    pub fn new(cmds_tree: Graph<String, ()>) -> CmdCompleter {
        CmdCompleter { cmds_tree }
    }

    fn get_subcommands(&self, node_idx: NodeIndex) -> Vec<Pair> {
        self.cmds_tree
            .neighbors_directed(node_idx, petgraph::Direction::Outgoing)
            .map(|idx| {
                let cmd = &self.cmds_tree[idx];
                Pair {
                    display: cmd.clone(),
                    replacement: cmd.clone(),
                }
            })
            .collect()
    }

    fn find_node_for_path(&self, parts: &[&str]) -> NodeIndex {
        let mut current_node = 0.into(); // Start at root
        
        for &part in parts {
            let found_neighbor = self.cmds_tree
                .neighbors_directed(current_node, petgraph::Direction::Outgoing)
                .find(|&neighbor| self.cmds_tree[neighbor] == part);
                
            match found_neighbor {
                Some(neighbor) => current_node = neighbor,
                None => break,
            }
        }
        
        current_node
    }

    fn complete_path(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let line_for_competion = &line[..pos];
        let parts: Vec<&str> = 
            InputParser::parse(Rule::cmd_full, line_for_competion)
                .map(|pairs| {
                    pairs
                        .into_iter()
                        .last()
                        .map(|pair| {
                            pair.into_inner()
                                .map(|p| match p.as_rule() {
                                    Rule::expr | Rule::cmd => p.as_str(),
                                    _ => unreachable!("{}", p)
                                })
                                .collect::<Vec<&str>>()
                        })
                        .unwrap_or(vec![])
                })
                .unwrap_or(vec![]);
        
        if parts.is_empty() {
            return Ok((0, self.get_subcommands(0.into())));
        }
        
        let ends_with_space = line[..pos].ends_with(' ');
        
        if ends_with_space {
            // All commands at the current path level
            let current_node = self.find_node_for_path(&parts);

            return Ok((pos, self.get_subcommands(current_node)));
        } else {
            // Filter commands that match partial input
            let last_part = parts.last().unwrap();
            let start = line[..pos].rfind(last_part).unwrap_or(0);
            let current_node = self.find_node_for_path(&parts[..parts.len()-1]);
            
            let completions = self.cmds_tree
                .neighbors_directed(current_node, petgraph::Direction::Outgoing)
                .filter_map(|idx| {
                    let cmd = &self.cmds_tree[idx];
                    if cmd.starts_with(last_part) {
                        Some(Pair {
                            display: cmd.clone(),
                            replacement: cmd.clone() + " ", // easier to compelete next part
                        })
                    } else {
                        None
                    }
                })
                .collect();
            
            return Ok((start, completions));
        }
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
        Owned(hint.bright_black().to_string())
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        self.highlighter.highlight_char(line, pos, kind)
    }
}

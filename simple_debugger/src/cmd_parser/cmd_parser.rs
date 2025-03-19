use clap::{command, CommandFactory, Parser, Subcommand};
use petgraph::Graph;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct CmdParser {
    #[command(subcommand)]
    command: Cmds,
}

use std::num::ParseIntError;
// hex parser
fn parse_hex(src: &str) -> Result<u32, ParseIntError> {
    if src.starts_with("0x") || src.starts_with("0X") {
        u32::from_str_radix(&src[2..], 16)
    } else {
        src.parse::<u32>()
    }
}

#[derive(Debug, Subcommand)]
#[command(author, version, about)]
pub enum Cmds {
    /// run single instrcution in the emulator
    SingleInstrcution {
        #[arg(default_value("1"))]
        count: u64,
    },

    /// continue the emulator
    Continue {},

    /// Times printf
    Times {},

    /// Get state info
    Info {
        #[command(subcommand)]
        subcmd: InfoCmds,

        // /// The target address(hex) and length
        // #[arg(value_parser = parse_hex)]
        // addr: u32,

        // /// Exam length in bitwidth, default as 1
        // #[arg(default_value("1"))]
        // length: u64,
    },
}

#[derive(Debug, Subcommand)]
#[command(author, version, about)]
pub enum InfoCmds {
    /// Get the state of the register
    Register {
        /// The target index and length
        index: u32,
    },

    /// Get the state of the memory
    Memory {
        /// The target address(hex) and length
        #[arg(value_parser = parse_hex)]
        addr: u32,

        /// Exam length in bitwidth, default as 1
        #[arg(default_value("1"))]
        length: u64,
    },
}

pub fn get_cmd_tree() -> Graph<String, ()> {
    let mut graph = Graph::<String, ()>::new();
    let root = graph.add_node("cmds".to_string());
    
    let help_node = graph.add_node("help".to_string());
    graph.add_edge(root, help_node, ());

    fn add_subcommands(graph: &mut Graph<String, ()>, parent: petgraph::graph::NodeIndex, cmd: &clap::Command) {
        for subcmd in cmd.get_subcommands() {
            let cmd_node = graph.add_node(subcmd.get_name().to_string());
            graph.add_edge(parent, cmd_node, ());
            
            // Recursively add this command's subcommands
            add_subcommands(graph, cmd_node, subcmd);
        }
    }
    
    // Start recursion from the root command
    add_subcommands(&mut graph, root, &CmdParser::command());

    graph
}

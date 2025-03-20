use clap::{command, CommandFactory, Parser, Subcommand, builder::styling};
use petgraph::Graph;

#[derive(Parser, Debug)]
#[command(author, version, about, styles = styling::Styles::styled()
.header(styling::AnsiColor::Green.on_default().bold())
.usage(styling::AnsiColor::Green.on_default().bold())
.literal(styling::AnsiColor::Blue.on_default().bold())
.placeholder(styling::AnsiColor::Cyan.on_default()))]
pub struct CmdParser {
    #[command(subcommand)]
    pub command: Cmds,
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
        #[command(subcommand)]
        subcmd: MemoryCmds,
    },
}

#[derive(Debug, Subcommand)]
pub enum MemoryCmds {
    /// show memory map
    ShowMemoryMap {},

    /// Examine memory
    Examine {
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

    fn add_subcommands(graph: &mut Graph<String, ()>, parent: petgraph::graph::NodeIndex, cmd: &clap::Command) {
        let subcommands: Vec<_> = cmd.get_subcommands().collect();
        if subcommands.is_empty() {
            return;
        }

        // every command with subcommand should also have a help command
        let help_node = graph.add_node("help".to_string());
        graph.add_edge(parent, help_node, ());

        for subcmd in subcommands {
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

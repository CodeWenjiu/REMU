use clap::{command, CommandFactory, Parser, Subcommand, builder::styling};
use petgraph::Graph;
use simulator::FunctionTarget;
use state::reg::RegIdentifier;

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

// reg identifier parser
fn parse_reg(src: &str) -> Result<RegIdentifier, ParseIntError> {
    if let Ok(index) = src.parse::<u32>() {
        Ok(RegIdentifier::Index(index))
    } else {
        Ok(RegIdentifier::Name(src.to_string()))
    }
}

#[derive(Debug, Subcommand)]
pub enum Cmds {
    /// step the emulator
    Step {
        #[command(subcommand)]
        subcmd: StepCmds,
    },

    /// continue the emulator
    Continue,

    /// Times printf
    Times,

    /// Get state info
    Info {
        #[command(subcommand)]
        subcmd: InfoCmds,
    },

    /// Differtest Reference
    Differtest {
        #[command(subcommand)]
        subcmd: DiffertestCmds,
    },

    /// Enable/Disable Simulator Function
    Function {
        #[command(subcommand)]
        subcmd: FunctionCmds,
    }
}

#[derive(Debug, Subcommand)]
pub enum StepCmds {
    /// Step n Cycles
    Cycles {
        /// The target cycle count
        #[arg(default_value("1"))]
        count: u64,
    },

    /// Step n instructions
    Instructions {
        /// The target instruction count
        #[arg(default_value("1"))]
        count: u64,
    },
}

#[derive(Debug, Subcommand)]
pub enum InfoCmds {
    /// Get the state of the register
    Register {
        /// The target register
        #[command(subcommand)]
        subcmd: Option<RegisterCmds>,
    },

    /// Get the state of the memory
    Memory {
        #[command(subcommand)]
        subcmd: MemoryCmds,
    },
}

#[derive(Debug, Subcommand)]
pub enum RegisterCmds {
    /// Show the state of the general purpose register
    GPR {
        /// The target register index
        #[arg(value_parser = parse_reg)]
        index: Option<RegIdentifier>,
    },
    
    /// Show the state of the control and status register
    CSR {
        /// The target register index
        #[arg(value_parser = parse_reg)]
        index: Option<RegIdentifier>,
    },
    
    /// Show the state of the Program Counter
    PC,
}

#[derive(Debug, Subcommand)]
pub enum MemoryCmds {
    /// show memory map
    ShowMemoryMap,

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

#[derive(Debug, Subcommand)]
pub enum DiffertestCmds {
    /// Get state info
    Info {
        #[command(subcommand)]
        subcmd: InfoCmds,
    },

    /// Set memory watch point
    MemWatchPoint {
        /// The target address(hex) and length
        #[arg(value_parser = parse_hex)]
        addr: u32,
    },
}

#[derive(Debug, Subcommand)]
pub enum FunctionCmds {
    /// Enable a function
    Enable {
        /// The target function
        #[command(subcommand)]
        subcmd: FunctionTarget,
    },

    /// Disable a function
    Disable {
        /// The target function
        #[command(subcommand)]
        subcmd: FunctionTarget,
    },
}

pub fn get_cmd_tree() -> Graph<String, ()> {
    let mut graph = Graph::<String, ()>::new();
    let root = graph.add_node("cmds".to_string());

    let help_node = graph.add_node("--version".to_string());
    graph.add_edge(root, help_node, ());

    fn add_subcommands(graph: &mut Graph<String, ()>, parent: petgraph::graph::NodeIndex, cmd: &clap::Command) {
        let subcommands: Vec<_> = cmd.get_subcommands().collect();
        
        // every command without subcommand should also have a `--help` command, otherwise it will have a `help` command
        if subcommands.is_empty() {
            let help_node = graph.add_node("--help".to_string());
            graph.add_edge(parent, help_node, ());
            return;
        }

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

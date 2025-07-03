use clap::{command, CommandFactory, Subcommand, builder::styling};
use petgraph::Graph;
use simulator::FunctionTarget;
use state::reg::RegIdentifier;

use std::num::ParseIntError;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, styles = styling::Styles::styled()
.header(styling::AnsiColor::Green.on_default().bold())
.usage(styling::AnsiColor::Green.on_default().bold())
.literal(styling::AnsiColor::Blue.on_default().bold())
.placeholder(styling::AnsiColor::Cyan.on_default()))]
pub struct CmdParser {
    #[command(subcommand)]
    pub command: Cmds,
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

    /// Set state
    Set {
        #[command(subcommand)]
        subcmd: SetCmds,
    },

    /// Print Expr Result
    Print {
        /// The target expression
        expr: String,
    },

    /// BreakPoint
    BreakPoint {
        #[command(subcommand)]
        subcmd: BreakPointCmds,
    },

    /// Enable/Disable Simulator Function
    Function {
        #[command(subcommand)]
        subcmd: FunctionCmds,
    },

    /// Differtest Reference
    Differtest {
        #[command(subcommand)]
        subcmd: DiffertestCmds,
    },
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
        subcmd: Option<RegisterInfoCmds>,
    },

    /// Get the state of the memory
    Memory {
        #[command(subcommand)]
        subcmd: MemorySetCmds,
    },

    /// Get the state of the pipeline
    Pipeline {
    },

    /// Get the state of the cache
    Cache {},

    /// Get extention info
    Extention {
        /// The target extention name
        key: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum SetCmds {
    /// Set the state of the register
    Register {
        #[command(subcommand)]
        subcmd: RegisterSetCmds,
    },

    /// Set the state of the memory
    Memory {
        /// The target address(expr)
        addr: String,

        /// The target value
        value: String,
    }
}

#[derive(Debug, Subcommand)]
pub enum BreakPointCmds {
    /// Set a breakpoint
    Add {
        /// The target address(expr)
        addr: String,
    },

    /// Remove a breakpoint
    Remove {
        /// The target address(expr)
        addr: String,
    },

    /// Show all breakpoints
    Show,
}

#[derive(Debug, Subcommand)]
pub enum RegisterInfoCmds {
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
pub enum RegisterSetCmds {
    /// Set the state of the Program Counter
    PC {
        /// The target value
        value: String,
    },

    /// Set the state of the general purpose register
    GPR {
        /// The target register index
        #[arg(value_parser = parse_reg)]
        index: RegIdentifier,

        /// The target value
        value: String,
    },

    /// Set the state of the control and status register
    CSR {
        /// The target register index
        #[arg(value_parser = parse_reg)]
        index: RegIdentifier,

        /// The target value
        value: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum MemorySetCmds {
    /// show memory map
    ShowMemoryMap,

    /// Examine memory
    #[clap(verbatim_doc_comment)]
    Examine {
        /// The target address(expr) and length
        addr: String,

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

    /// Set state
    Set {
        #[command(subcommand)]
        subcmd: SetCmds,
    },

    /// Set memory watch point
    MemWatchPoint {
        /// The target address(expr) and length
        addr: Option<String>,
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

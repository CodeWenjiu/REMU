use clap::{CommandFactory, builder::styling};
use petgraph::graph::{Graph, NodeIndex};
use remu_fmt::parse_prefixed_uint;
use remu_harness::{FuncCmd, StateCmd, StatCmd};

fn populate_graph(cmd: &clap::Command, graph: &mut Graph<String, ()>, parent: NodeIndex) {
    let mut has_children = false;

    for sub in cmd.get_subcommands() {
        has_children = true;
        let idx = graph.add_node(sub.get_name().to_string());
        graph.add_edge(parent, idx, ());
        populate_graph(sub, graph, idx);
    }

    // For any node that has subcommands, also add an implicit `help` child
    if has_children {
        let help_idx = graph.add_node("help".to_string());
        graph.add_edge(parent, help_idx, ());
    }
}

/// Build a command graph for hierarchical completion.
/// Returns the graph and the root node index.
pub fn get_command_graph() -> (Graph<String, ()>, NodeIndex) {
    let mut graph = Graph::<String, ()>::new();
    let root = graph.add_node(env!("CARGO_PKG_NAME").to_string());
    let command = DebuggerCommand::command();
    populate_graph(&command, &mut graph, root);
    (graph, root)
}

#[derive(clap::Parser, Debug)]
#[command(
    author,
    version,
    about,
    disable_help_flag = true,
    disable_version_flag = true,
    styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default())
)]
pub struct DebuggerCommand {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// continue the emulator
    Continue,

    /// Step
    Step {
        /// Number of steps to take
        #[arg(default_value_t = 1)]
        times: usize,
    },

    /// Func Command
    Func {
        #[command(subcommand)]
        subcmd: FuncCmd,
    },

    /// State Command (DUT / remu)
    State {
        #[command(subcommand)]
        subcmd: StateCmd,
    },

    /// Reference model state (Spike): same subcommands as `state`, executed on the ref simulator
    RefState {
        #[command(subcommand)]
        subcmd: StateCmd,
    },

    /// Breakpoint Command
    Breakpoint {
        #[command(subcommand)]
        subcmd: BreakpointCmd,
    },

    /// Stat Command
    Stat {
        #[command(subcommand)]
        subcmd: StatCmd,
    },

    /// Quit the debugger
    Quit,
}

#[derive(Debug, clap::Subcommand)]
pub enum BreakpointCmd {
    /// Set breakpoint at address (stop when PC hits this address)
    Set {
        /// Breakpoint address (0x/0o/0b/0d prefix or decimal, e.g. 0x80000000)
        #[arg(value_parser = parse_prefixed_uint::<u32>)]
        addr: u32,
    },
    /// Delete breakpoint at address
    Del {
        /// Breakpoint address (0x/0o/0b/0d prefix or decimal, e.g. 0x80000000)
        #[arg(value_parser = parse_prefixed_uint::<u32>)]
        addr: u32,
    },
    /// Print all breakpoints
    Print,
}

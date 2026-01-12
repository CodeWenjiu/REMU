use clap::{CommandFactory, Subcommand, builder::styling};
use petgraph::graph::{Graph, NodeIndex};

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
pub(crate) struct CommandParser {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Print version information
    Version,

    /// continue the emulator
    Continue,

    /// Times printf
    Times {
        #[command(subcommand)]
        subcmd: TimeCmds,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum TimeCmds {
    /// Times Count
    Count {
        #[command(subcommand)]
        subcmd: TimeCountCmds,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum TimeCountCmds {
    Test,
}

fn populate_graph(cmd: &clap::Command, graph: &mut Graph<String, ()>, parent: NodeIndex) {
    for sub in cmd.get_subcommands() {
        let idx = graph.add_node(sub.get_name().to_string());
        graph.add_edge(parent, idx, ());
        populate_graph(sub, graph, idx);
    }
}

/// Build a command graph for hierarchical completion.
/// Returns the graph and the root node index.
pub fn get_command_graph() -> (Graph<String, ()>, NodeIndex) {
    let mut graph = Graph::<String, ()>::new();
    let root = graph.add_node(env!("CARGO_PKG_NAME").to_string());
    let command = CommandParser::command();
    populate_graph(&command, &mut graph, root);
    (graph, root)
}

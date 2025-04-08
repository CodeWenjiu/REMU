use std::{fs, io::{BufRead, Write}};

use petgraph::{graph::NodeIndex, Graph};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub replacement: String,
}

pub struct CmdCompleter {
    cmds_tree: Graph<String, ()>,
}

impl CmdCompleter {
    pub fn new(cmds_tree: Graph<String, ()>) -> Self {
        Self { cmds_tree }
    }

    pub fn get_subcommands(&self, node_idx: NodeIndex) -> Vec<Pair> {
        self.cmds_tree
            .neighbors_directed(node_idx, petgraph::Direction::Outgoing)
            .map(|idx| {
                let cmd = &self.cmds_tree[idx];
                Pair {
                    replacement: cmd.clone() + " ",
                }
            })
            .collect()
    }

    pub fn find_node(&self, current_node: NodeIndex, part: &str) -> Option<NodeIndex> {
        self.cmds_tree
            .neighbors_directed(current_node, petgraph::Direction::Outgoing)
            .find(|&neighbor| self.cmds_tree[neighbor] == part)
    }

    pub fn find_node_for_path(&self, parts: &[&str]) -> NodeIndex {
        let mut current_node = 0.into();

        for &part in parts {
            current_node = match self.find_node(current_node, part) {
                Some(neighbor) => neighbor,
                None => return current_node,
            };
        }

        current_node
    }

    pub fn validate(&self, line: &str) -> bool {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return true;
        }
        
        let mut current_node = 0.into();
        for part in parts {
            current_node = match self.find_node(current_node, part) {
                Some(neighbor) => neighbor,
                None => return false,
            };
        }

        true
    }

    pub fn complete_path(&self, line: &str, pos: usize) -> (usize, Vec<Pair>) {
        let parts: Vec<&str> = line[..pos].split_whitespace().collect();

        if parts.is_empty() {
            return (0, self.get_subcommands(0.into()).into_iter().collect());
        }

        let ends_with_space = line[..pos].ends_with(' ');

        if ends_with_space {
            let current_node = self.find_node_for_path(&parts);
            return (pos, self.get_subcommands(current_node).into_iter().collect());
        }

        let last_part = parts.last().unwrap();
        let start = line[..pos].rfind(last_part).unwrap_or(0);
        let current_node = self.find_node_for_path(&parts[..parts.len() - 1]);

        let completions = self
            .cmds_tree
            .neighbors_directed(current_node, petgraph::Direction::Outgoing)
            .filter_map(|idx| {
                let cmd = &self.cmds_tree[idx];
                if cmd.starts_with(last_part) {
                    Some(Pair {
                        replacement: format!("{} ", cmd), // 确保 replacement 包含空格
                    })
                } else {
                    None
                }
            })
            .collect();

        (start, completions)
    }
}

pub struct HistoryCompeleter {
    command_history: (isize, Vec<String>),
    ghost_text: String,
    history_file_path: String,
    max_history_length: usize,
}

impl HistoryCompeleter {
    pub fn new(max_history_length: usize) -> Self {
        let history_file_path = "command_history".to_string();
        
        let command_history = match Self::load_history_from_file(&history_file_path) {
            Ok(history) => history,
            Err(_) => (-1, Vec::new()),
        };

        Self {
            command_history,
            ghost_text: String::new(), // for histroy completion
            history_file_path,
            max_history_length,
        }
    }

    fn load_history_from_file(file_path: &str) -> Result<(isize, Vec<String>), Box<dyn std::error::Error>> {
        if std::path::Path::new(file_path).exists() {
            let file = std::fs::File::open(file_path)?;
            let reader = std::io::BufReader::new(file);
            let lines: Result<Vec<String>, _> = reader.lines().collect();
            match lines {
                Ok(lines) => Ok((-1, lines)),
                Err(e) => {
                    eprintln!("Failed to read history from file: {}", e);
                    Ok((-1, Vec::new()))
                }
            }
        } else {
            Ok((-1, Vec::new()))
        }
    }

    pub fn run_completion(&mut self, current_line: String) {
        if current_line.is_empty() {
            self.ghost_text.clear();
            return;
        }

        if let Some(history_entry) = self.command_history
            .1
            .iter()
            .rev() 
            .find(|history_entry| history_entry.starts_with(&current_line))
        {
            // 计算需要补全的部分
            self.ghost_text = history_entry[current_line.len()..].to_string();
        } else {
            self.ghost_text.clear();
        }
    }

    pub fn cmd_execute(&mut self, command: &String) {
        let (history_index, command_history) = &mut self.command_history;
        if command_history.last() == Some(command) {
            return;
        }
        command_history.push(command.clone());
        *history_index = -1;
    }

    pub fn get_history_up(&mut self) -> Option<&String> {
        let (history_index, command_history) = &mut self.command_history;

        if command_history.is_empty() {
            return None;
        }

        if *history_index == -1 {
            *history_index = (command_history.len() - 1) as isize;
        } else {
            *history_index = (*history_index - 1).max(0);
        }

        Some(&command_history[*history_index as usize])
    }

    pub fn get_history_down(&mut self) -> Option<&String> {
        let (history_index, command_history) = &mut self.command_history;

        if command_history.is_empty() {
            return None;
        }

        if *history_index == -1 {
            return None;
        } else {
            *history_index = (*history_index + 1).min((command_history.len() - 1) as isize);
        }

        Some(&command_history[*history_index as usize])
    }

    pub fn suggestions(&self) -> &String {
        &self.ghost_text
    }

    pub fn compelete(&mut self) -> String {
        std::mem::take(&mut self.ghost_text)
    }
}

impl Drop for HistoryCompeleter {
    fn drop(&mut self) {
        let mut file = fs::File::create(self.history_file_path.clone()).unwrap();
        let history = &self.command_history.1;
        let history = if history.len() > self.max_history_length {
            history[history.len() - self.max_history_length..].to_vec()
        } else {
            history.clone()
        };
        for command in history {
            file.write_all(command.as_bytes()).unwrap();
            file.write_all(b"\n").unwrap();
        }
    }
}

pub struct SuggestionManager {
    suggestions: Vec<Pair>,
    selected_idx: usize,
    start_pos: usize,
    visible: bool,
}

impl SuggestionManager {
    pub fn new() -> Self {
        Self {
            suggestions: Vec::new(),
            selected_idx: 0,
            start_pos: 0,
            visible: false,
        }
    }

    pub fn set_suggestions(&mut self, suggestions: Vec<Pair>, start_pos: usize) {
        self.suggestions = suggestions;
        self.start_pos = start_pos;
        self.selected_idx = 0;
        self.visible = !self.suggestions.is_empty();
    }

    pub fn clear(&mut self) {
        self.suggestions.clear();
        self.visible = false;
    }

    pub fn cycle(&mut self, forward: bool) {
        if !self.visible || self.suggestions.is_empty() {
            return;
        }

        let count = self.suggestions.len();
        self.selected_idx = (self.selected_idx + if forward { 1 } else { count - 1 }) % count;
    }

    pub fn selected(&self) -> Option<&Pair> {
        if self.visible && !self.suggestions.is_empty() {
            self.suggestions.get(self.selected_idx)
        } else {
            None
        }
    }
}
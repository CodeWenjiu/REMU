use std::cell::RefCell;
use std::rc::Rc;
use std::{collections::HashMap, fmt::Display};
use std::hash::Hash;
use std::fmt::{Debug, Error};

use petgraph::{graph::NodeIndex, algo::toposort, Graph};
use remu_macro::log_error;
use logger::Logger;
use remu_utils::{ProcessError, ProcessResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BaseStageCell {
    Input,
    IfId,
    IdIs,
    IsAl,
    IsLs,
    ExWb,
}

impl Display for BaseStageCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            BaseStageCell::Input => write!(f, "Input"),
            BaseStageCell::IfId => write!(f, "IfId"),
            BaseStageCell::IdIs => write!(f, "IdIs"),
            BaseStageCell::IsAl => write!(f, "IsAl"),
            BaseStageCell::IsLs => write!(f, "IsLs"),
            BaseStageCell::ExWb => write!(f, "ExWb"),
        }
    }
}

#[derive(Debug, Clone)]
struct MessageChannel {
    buffer: Rc::<RefCell::<Vec<(u32, u32)>>>,
    capacity: usize,
    transmit_target: Option<BaseStageCell>, // Use generic type T
}

impl MessageChannel {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: Rc::new(RefCell::new(Vec::new())),
            capacity,
            transmit_target: None,
        }
    }

    fn push(&mut self, data: (u32, u32)) -> ProcessResult<()> {
        if self.buffer.borrow().len() < self.capacity {
            self.buffer.borrow_mut().push(data);
            Ok(())
        } else {
            Err(ProcessError::Recoverable)
        }
    }

    fn flush(&mut self) {
        self.buffer.borrow_mut().clear();
        self.transmit_target = None;
    }
}

#[derive(Debug, Clone)]
pub struct ModelCell {
    channel: MessageChannel,
    node_index: NodeIndex,
}

#[derive(Debug, Clone)]
pub struct StageModel { 
    pub cells: HashMap<BaseStageCell, ModelCell>, 
    graph: Graph<BaseStageCell, ()>, 
}

impl StageModel {
    fn find_cell(&mut self, find_type: BaseStageCell) -> ProcessResult<&mut ModelCell> {
        self.cells.get_mut(&find_type).ok_or({
            ProcessError::Recoverable
        }).map_err(|e| {
            log_error!(format!("Cell {:?} not found", find_type));
            e
        })
    }

    pub fn send(&mut self, data: (u32, u32), to: BaseStageCell) -> ProcessResult<()> {
        self.find_cell(BaseStageCell::Input)?
            .channel
            .push(data)
            .map_err(|e| {
                log_error!(format!("{:?}: buffer is full", BaseStageCell::Input));
                e
            })?;

        self.trans(BaseStageCell::Input, to)
    }

    pub fn trans(&mut self, from: BaseStageCell, to: BaseStageCell) -> ProcessResult<()> {
        let from_index = self.find_cell(from)?.node_index;
        let to_index = self.find_cell(to)?.node_index;

        if !self.graph.contains_edge(from_index.into(), to_index.into()) {
            log_error!(format!("Cells {:?} and {:?} are not connected", from, to));
            return Err(ProcessError::Recoverable);
        }
        
        let from_cell = self.find_cell(from)?;
        from_cell.channel.transmit_target = Some(to);

        Ok(())
    }

    pub fn get(&mut self) -> ProcessResult<(u32, u32)> {
        let output = BaseStageCell::ExWb; 
        let channel = &mut self.find_cell(output)?.channel;
        let data = channel.buffer.borrow_mut().pop().ok_or({
            ProcessError::Recoverable
        }).map_err(|e|{
            log_error!(format!("{:?}: buffer is empty", output)); // 使用局部变量 output
            e
        })?;

        Ok(data)
    }

    pub fn fetch(&mut self, from: BaseStageCell) -> ProcessResult<(u32, u32)> {

        let buffer = self.find_cell(from)?.channel.buffer.borrow();

        let data = buffer.last().ok_or({
            ProcessError::Recoverable
        }).map_err(|e|{
            log_error!(format!("{:?}: buffer is empty", from));
            e
        })?;

        Ok(*data)
    } 

    pub fn update(&mut self) -> ProcessResult<()> {
        let order = toposort(&self.graph, None)
            .map_err(|_| {log_error!("WTF"); ProcessError::Fatal})?;

        for &node in order.iter().rev() {
            let channel = self.graph[node];
            let transmit_target;
            let from_node;
            {
                let channel_obj = self.find_cell(channel)?;
                transmit_target = channel_obj.channel.transmit_target.take();
                from_node = channel_obj.node_index;
            }
            if let Some(to) = transmit_target {
                let to_node = self.find_cell(to)?.node_index;
                if !self.graph.contains_edge(from_node, to_node) {
                    log_error!(format!("{:?} and {:?} are not connected", channel, to));
                    return Err(ProcessError::Fatal);
                }
                
                let channel_obj = self.find_cell(channel)?;
                let data = {
                    channel_obj.channel.buffer.borrow_mut().pop().ok_or({
                        ProcessError::Recoverable
                    })
                }.map_err(|e| {
                    log_error!(format!("{:?} buffer is empty to {:?}", channel, to));
                    e
                })?;
                
                let target_channel = self.find_cell(to)?;
                target_channel.channel.push(data).map_err(|e| {
                    log_error!(format!("Buffer {:?} overflow from {:?}", to, channel));
                    e
                })?;
            }
        }

        Ok(())
    }

    pub fn flush(&mut self) {
        for (_, channel) in self.cells.iter_mut() {
            channel.channel.flush();
        }
    }
}

impl Default for StageModel {
    fn default() -> Self {
        let mut graph = Graph::new();
        let mut cells = HashMap::new();

        let input = BaseStageCell::Input;
        let input_node = graph.add_node(input);
        cells.insert(
            input,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: input_node,
            },
        );

        let idu = BaseStageCell::IfId;
        let idu_node = graph.add_node(idu);
        graph.add_edge(input_node, idu_node, ());
        cells.insert(
            idu,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: idu_node,
            },
        );

        let isu = BaseStageCell::IdIs;
        let isu_node = graph.add_node(isu);
        graph.add_edge(idu_node, isu_node, ());
        cells.insert(
            isu,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: isu_node,
            },
        );

        let alu = BaseStageCell::IsAl;
        let alu_node = graph.add_node(alu);
        graph.add_edge(isu_node, alu_node, ());
        cells.insert(
            alu,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: alu_node,
            },
        );

        let lsu = BaseStageCell::IsLs;
        let lsu_node = graph.add_node(lsu);
        graph.add_edge(isu_node, lsu_node, ());
        cells.insert(
            lsu,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: lsu_node,
            },
        );

        let output = BaseStageCell::ExWb;
        let output_node = graph.add_node(output);
        graph.add_edge(alu_node, output_node, ());
        graph.add_edge(lsu_node, output_node, ());
        cells.insert(
            output,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: output_node,
            },
        );

        Self {
            cells,
            graph,
        }
    }
}

impl Display for StageModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let order = toposort(&self.graph, None)
            .map_err(|_| {log_error!("WTF"); Error})?
            .into_iter()
            .filter(|t| self.graph[*t] != BaseStageCell::Input)
            .collect::<Vec<_>>();

        let mut cells_str = String::new();
        for &node in &order {
            let cell_type = self.graph[node];
            if let Some(model_cell) = self.cells.get(&cell_type) {
                // 格式化缓冲区内容（如 [(0x1234, 0x5678)]）
                let buffer = model_cell.channel.buffer
                    .borrow()
                    .iter()
                    .map(|&(a, b)| format!("(0x{:08x}, 0x{:08x})", a, b))
                    .collect::<Vec<_>>()
                    .join(", ");
                cells_str.push_str(&format!("  {}: [{}]\n", 
                    cell_type, buffer));
            }
        }

        write!(f, 
            "Pipeline Model\n\
             ==============\n\
             Processing Cells:\n{}\n",
            cells_str
        )
    }
}

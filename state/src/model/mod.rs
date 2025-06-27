use std::cell::RefCell;
use std::rc::Rc;
use std::{collections::HashMap, fmt::Display};
use std::hash::Hash;
use std::fmt::{Debug, Error};

use petgraph::{graph::NodeIndex, algo::toposort, Graph};
use remu_macro::log_error;
use remu_utils::{ItraceConfigtionalWrapper, ProcessError, ProcessResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BaseStageCell {
    Input,
    BpIf,
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
            BaseStageCell::BpIf => write!(f, "BpIf"),
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
    conditional: ItraceConfigtionalWrapper,

    is_flush: bool,
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

    pub fn instruction_fetch(&mut self, inst: u32) -> ProcessResult<()> {
        let mut buffer = self.find_cell(BaseStageCell::BpIf)?
            .channel
            .buffer
            .borrow_mut();
        
        if buffer.is_empty() {
            log_error!(format!("{:?}: buffer is empty", BaseStageCell::BpIf));
            return Err(ProcessError::Recoverable);
        }
        
        buffer[0].1 = inst;
        drop(buffer);

        self.trans(BaseStageCell::BpIf, BaseStageCell::IfId)
    }

    pub fn cell_input(&mut self, data: (u32, u32), to: BaseStageCell) -> ProcessResult<()> {
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

        if self.is_flush {
            self.do_flush();
            self.is_flush = false;
        } else {
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
        }

        Ok(())
    }

    fn do_flush(&mut self) {
        for (_, channel) in self.cells.iter_mut() {
            channel.channel.flush();
        }
    }

    pub fn flush(&mut self) {
        self.is_flush = true;
    }

    pub fn check(&self, dut_models: &StageModel) -> ProcessResult<()> {
        for (cell_type, dut_cell) in dut_models.cells.iter() {
            let dut_channel = dut_cell.channel.buffer.borrow();
            let ref_channel = self.cells.get(cell_type).unwrap().channel.buffer.borrow();
            if dut_channel.len() != ref_channel.len() {
                log_error!(format!("Cell {:?} buffer length mismatch: dut: {}, ref: {}", cell_type, dut_channel.len(), ref_channel.len()));
                return Err(ProcessError::Recoverable);
            }
            for (dut_data, ref_data) in dut_channel.iter().zip(ref_channel.iter()) {
                if dut_data != ref_data {
                    log_error!(format!("Cell {:?} buffer data mismatch: dut: {:?}, ref: {:?}", cell_type, dut_data, ref_data));
                    return Err(ProcessError::Recoverable);
                }
            }
        }
        Ok(())
    }

    pub fn with_branchpredict(conditional: ItraceConfigtionalWrapper) -> Self {
        let mut graph = Graph::new();
        let mut cells = HashMap::new();

        let input = BaseStageCell::Input;
        let bpif = BaseStageCell::BpIf;
        let ifid = BaseStageCell::IfId;
        let idis = BaseStageCell::IdIs;
        let isal = BaseStageCell::IsAl;
        let isls = BaseStageCell::IsLs;
        let exwb = BaseStageCell::ExWb;

        let input_node = graph.add_node(input);
        let bpif_node = graph.add_node(bpif);
        let ifid_node = graph.add_node(ifid);
        let idis_node = graph.add_node(idis);
        let isal_node = graph.add_node(isal);
        let isls_node = graph.add_node(isls);
        let exwb_node = graph.add_node(exwb);

        graph.add_edge(input_node, bpif_node, ());
        graph.add_edge(bpif_node, ifid_node, ());
        graph.add_edge(idis_node, isal_node, ());
        graph.add_edge(ifid_node, idis_node, ());
        graph.add_edge(idis_node, isls_node, ());
        graph.add_edge(isal_node, exwb_node, ());
        graph.add_edge(isls_node, exwb_node, ());

        cells.insert(
            input,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: input_node,
            },
        );

        cells.insert(
            bpif,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: bpif_node,
            },
        );

        cells.insert(
            ifid,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: ifid_node,
            },
        );

        cells.insert(
            isal,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: isal_node,
            },
        );
        
        cells.insert(
            idis,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: idis_node,
            },
        );

        cells.insert(
            isls,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: isls_node,
            },
        );

        cells.insert(
            exwb,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: exwb_node,
            },
        );

        Self {
            cells,
            graph,
            conditional,

            is_flush: false,
        }
    }

    pub fn default(conditional: ItraceConfigtionalWrapper) -> Self {
        let mut graph = Graph::new();
        let mut cells = HashMap::new();

        let input = BaseStageCell::Input;
        let ifid = BaseStageCell::IfId;
        let idis = BaseStageCell::IdIs;
        let isal = BaseStageCell::IsAl;
        let isls = BaseStageCell::IsLs;
        let exwb = BaseStageCell::ExWb;

        let input_node = graph.add_node(input);
        let ifid_node = graph.add_node(ifid);
        let idis_node = graph.add_node(idis);
        let isal_node = graph.add_node(isal);
        let isls_node = graph.add_node(isls);
        let exwb_node = graph.add_node(exwb);

        graph.add_edge(input_node, ifid_node, ());
        graph.add_edge(idis_node, isal_node, ());
        graph.add_edge(ifid_node, idis_node, ());
        graph.add_edge(idis_node, isls_node, ());
        graph.add_edge(isal_node, exwb_node, ());
        graph.add_edge(isls_node, exwb_node, ());

        cells.insert(
            input,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: input_node,
            },
        );

        cells.insert(
            ifid,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: ifid_node,
            },
        );

        cells.insert(
            isal,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: isal_node,
            },
        );
        
        cells.insert(
            idis,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: idis_node,
            },
        );

        cells.insert(
            isls,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: isls_node,
            },
        );

        cells.insert(
            exwb,
            ModelCell {
                channel: MessageChannel::new(1),
                node_index: exwb_node,
            },
        );

        Self {
            cells,
            graph,
            conditional,

            is_flush: false,
        }
    }

    // pub fn with_branchpredition
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
                    .map(|&(a, b)| format!("(0x{:08x}, {})", a, self.conditional.try_analize_fmt(b, a)))
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

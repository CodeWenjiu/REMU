use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::fmt::Debug;
use std::rc::Rc;

use bitflags::bitflags;
use petgraph::{graph::NodeIndex, algo::toposort, Graph};
use petgraph::graph::DefaultIx;
use remu_macro::log_error;
use logger::Logger;
// Assuming Logger is available, otherwise add `use logger::Logger;`
use remu_utils::{ProcessError, ProcessResult};
// Removed bitflags! macro and PipeCell definition

#[derive(Debug, Clone)]
struct MessageChannel<T: Debug + Eq + Hash + Clone + Copy> {
    buffer: Vec<(u32, u32)>,
    capacity: usize,
    transmit_target: Option<T>, // Use generic type T
}

impl<T: Debug + Eq + Hash + Clone + Copy> MessageChannel<T> {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::new(),
            capacity,
            transmit_target: None,
        }
    }

    fn push(&mut self, data: (u32, u32)) -> ProcessResult<()> {
        if self.buffer.len() < self.capacity {
            self.buffer.push(data);
            Ok(())
        } else {
            Err(ProcessError::Recoverable)
        }
    }

    fn flush(&mut self) {
        self.buffer.clear();
        self.transmit_target = None;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct BasePipeCell: u32 {
        const IFU = 1 << 0;
        const IDU = 1 << 1;
        const ALU = 1 << 2;
        const AGU = 1 << 3;
        const LSU = 1 << 4;
        const WBU = 1 << 5;
    }
}

#[derive(Debug, Clone)]
pub struct PipelineModel<T: Debug + Eq + Hash + Clone + Copy> { // Made generic over T
    channels: HashMap<T, (Rc<RefCell<MessageChannel<T>>>, NodeIndex<DefaultIx>)>, // Use T
    graph: Graph<T, ()>, // Use T as node weight
    input: T, // Use T
    output: T,
}

impl<T: Debug + Eq + Hash + Clone + Copy> PipelineModel<T> {
    pub fn send(&mut self, data: (u32, u32), to: T) -> ProcessResult<()> {
        self.channels
            .get_mut(&self.input)
            .unwrap()
            .0
            .borrow_mut()
            .push(data)
            .map_err(|e| {
                log_error!(format!("{:?}: buffer is full", self.input));
                e
            })?;
        
        self.trans(self.input, to);

        Ok(())
    }

    pub fn trans(&mut self, from: T, to: T) {
        if !self.check_connect(from, to) {
            log_error!(format!("{:?} and {:?} are not connected", from, to));
            return;
        }
        
        self.channels
            .get_mut(&from)
            .unwrap()
            .0
            .borrow_mut()
            .transmit_target = Some(to);
    }

    pub fn check_connect(&self, from: T, to: T) -> bool {
        // check if from and to are connected
        if self.graph.contains_edge(self.channels.get(&from).unwrap().1, self.channels.get(&to).unwrap().1) {
            return true;
        }
        false
    }

    pub fn get(&mut self) -> ProcessResult<(u32, u32)> {
        let channel = &mut self.channels.get_mut(&self.output).unwrap().0;
        let data = channel.borrow_mut().buffer.pop().ok_or({
            ProcessError::Recoverable
        }).map_err(|e|{
            log_error!(format!("{:?}: buffer is empty", self.output));
            e
        })?;

        Ok(data)
    }

    pub fn fetch(&mut self, from: T) -> ProcessResult<(u32, u32)> {
        let channel = &mut self.channels.get_mut(&from).unwrap().0;
        let binding = channel.borrow_mut();
        let data = binding.buffer.last().ok_or({
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
                let channel_obj = self.channels.get_mut(&channel).unwrap();
                transmit_target = channel_obj.0.borrow_mut().transmit_target.take();
                from_node = channel_obj.1;
            }
            if let Some(to) = transmit_target {
                let to_node = self.channels.get(&to).unwrap().1;
                if !self.graph.contains_edge(from_node, to_node) {
                    log_error!(format!("{:?} and {:?} are not connected", channel, to));
                    return Err(ProcessError::Fatal);
                }
                
                let channel_obj = self.channels.get_mut(&channel).unwrap();
                let data = {
                    channel_obj.0.borrow_mut().buffer.pop().ok_or({
                        ProcessError::Recoverable
                    })
                }.map_err(|e| {
                    log_error!(format!("{:?} buffer is empty to {:?}", channel, to));
                    e
                })?;
                
                let target_channel = self.channels.get_mut(&to).unwrap();
                target_channel.0.borrow_mut().push(data).map_err(|e| {
                    log_error!(format!("Buffer {:?} overflow from {:?}", to, channel));
                    e
                })?;
            }
        }

        Ok(())
    }

    pub fn flush(&mut self) {
        for (_, (channel, _)) in self.channels.iter_mut() {
            channel.borrow_mut().flush();
        }
    }
}

impl PipelineModel<BasePipeCell> {
    pub fn new() -> Self {
        let mut graph = Graph::new(); // 显式指定 Graph 的 NodeIndex 类型
        let mut channels = HashMap::new();

        let input = BasePipeCell::IFU;
        let input_node = graph.add_node(input);
        channels.insert(input, (Rc::new(RefCell::new(MessageChannel::new(1))), input_node));

        let idu = BasePipeCell::IDU;
        let idu_node = graph.add_node(idu);
        graph.add_edge(input_node, idu_node, ());
        channels.insert(idu, (Rc::new(RefCell::new(MessageChannel::new(1))), idu_node));

        let alu = BasePipeCell::ALU;
        let alu_node = graph.add_node(alu);
        graph.add_edge(idu_node, alu_node, ());
        channels.insert(alu, (Rc::new(RefCell::new(MessageChannel::new(1))), alu_node));

        let agu = BasePipeCell::AGU;
        let agu_node = graph.add_node(agu);
        graph.add_edge(idu_node, agu_node, ());
        channels.insert(agu, (Rc::new(RefCell::new(MessageChannel::new(1))), agu_node));

        let lsu = BasePipeCell::LSU;
        let lsu_node = graph.add_node(lsu);
        graph.add_edge(agu_node, lsu_node, ());
        channels.insert(lsu, (Rc::new(RefCell::new(MessageChannel::new(1))), lsu_node));

        let output = BasePipeCell::WBU;
        let output_node = graph.add_node(output);
        graph.add_edge(alu_node, output_node, ());
        graph.add_edge(lsu_node, output_node, ());
        channels.insert(output, (Rc::new(RefCell::new(MessageChannel::new(1))), output_node));

        Self {
            channels,
            graph,
            input,
            output,
        }
    }
}
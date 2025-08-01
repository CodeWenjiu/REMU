use std::{cell::RefCell, rc::Rc};

use comfy_table::Table;
use remu_macro::log_error;
use remu_utils::ProcessError;

use crate::cache::{CacheConfiguration, CacheTable, CacheBase, Replacement};

#[derive(Clone, Debug)]
pub struct BtbMeta {
    tag: u32,
}

impl BtbMeta {
    fn new() -> Self {
        Self {
            tag: 0
        }
    }
}

#[derive(Clone, Debug)]
pub struct BdbData {
    pub counter: u32,
}

impl BdbData {
    fn new() -> Self {
        Self {
            counter: 0
        }
    }
}

#[derive(Clone, Debug)]
pub struct BtbData {
    pub typ: bool,
    pub target: u32,
}

impl BtbData {
    fn new() -> Self {
        Self {
            typ: false,
            target: 0
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct BRMsg {
    pub br_type: bool,
    pub br_dir: bool,
}

#[derive(Debug, Clone)]
pub struct BTB {
    table: CacheTable,

    meta: Rc<RefCell<Vec<Vec<BtbMeta>>>>,
    data: Rc<RefCell<Vec<BtbData>>>,

    bdb: Rc<RefCell<Vec<BdbData>>>,

    replacement: Replacement,
}

impl BTB {
    pub fn hyper_replace(&mut self, addr: u32, brmsg: BRMsg)  {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);

        let way = self.replacement.way(set);
        self.base_meta_write(set, way, tag);
        
        let data_index = self.table.get_data_line_index(set, way);
        let data_block = &mut self.bdb.borrow_mut()[data_index as usize];

        if brmsg.br_type {
            if brmsg.br_dir {
                if data_block.counter < 3 {
                    data_block.counter += 1;
                }
            } else {
                if data_block.counter > 0 {
                    data_block.counter -= 1;
                }
            }
        }
    }
}

impl CacheBase for BTB {
    type CacheData = BtbData;

    fn new(config: CacheConfiguration) -> Self {
        let (set,way, block_num, replacement) = (
            config.set,
            config.way,
            config.block_num,
            &config.replacement,
        );

        let table = CacheTable::new(set, way, block_num);

        BTB {
            table,

            meta: Rc::new(RefCell::new(vec![vec![BtbMeta::new(); way as usize]; set as usize])),
            data: Rc::new(RefCell::new(vec![BtbData::new(); (set * way) as usize])), // BTB should not have block_num

            bdb: Rc::new(RefCell::new(vec![BdbData::new(); (set * way) as usize])), 

            replacement: Replacement::new(set, way, replacement),
        }
    }

    fn base_meta_write(&mut self, set: u32, way: u32, tag: u32) {
        let meta = &mut self.meta.borrow_mut()[set as usize][way as usize];

        // Update the metadata
        meta.tag = tag;
    }

    fn base_data_write(&mut self, set: u32, way: u32, block_num: u32, data: Self::CacheData) {
        let _ = block_num;
        let data_index = self.table.get_data_line_index(set, way);
        let data_block = &mut self.data.borrow_mut()[data_index as usize];

        // Update the data block
        *data_block = data;
    }

    fn base_read(&self, set: u32, way: u32) -> Vec<BtbData> {
        let data_index = self.table.get_data_line_index(set, way);
        let data_block = self.data.borrow()[data_index as usize].clone();

        vec![data_block]
    }

    fn read(&mut self, addr: u32) -> Option<Vec<BtbData>> {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);

        let way = {
            self.meta.borrow()[set as usize]
                .iter()
                .position(|meta_block| meta_block.tag == tag)
        };

        way.and_then(|way| {
            self.replacement.access(set, way as u32);
            
            let data = self.base_read(set, way as u32);
            
            // let data_index = self.table.get_data_line_index(set, way as u32);
            // if data[0].typ && self.bdb.borrow()[data_index].counter > 1 {
                Some(data)
            // } else {
            //     None
            // }
        })
    }

    fn replace(&mut self, addr: u32, data: Vec<Self::CacheData>) -> Option<(u32, Vec<Self::CacheData>)> {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);

        let way = self.replacement.way(set);
        self.replacement.access(set, way);
        self.base_meta_write(set, way, tag);

        let block_num = 0;
        self.base_data_write(set, way, block_num, data[0].clone());

        None
    }

    fn print(&self) {
        let mut table = Table::new();

        for (set_idx, meta_line) in self.meta.borrow().iter().enumerate() {
            let mut row = vec![format!("Set {}", set_idx)];

            for (way_idx, meta_block) in meta_line.iter().enumerate() {
                let data_block = &self.data.borrow()[(set_idx * self.meta.borrow()[0].len()) + way_idx];
                let bdb_block = &self.bdb.borrow()[(set_idx * self.meta.borrow()[0].len()) + way_idx];
                row.push(format!(
                    "Way {}: Tag: {:#08x}, Target: {:#08x}, Counter: {}",
                    way_idx, meta_block.tag, data_block.target, bdb_block.counter
                ));
            }

            table.add_row(row);
        }

        println!("{table}");
    }

    fn test(&self, dut: &Self) -> remu_utils::ProcessResult<()> {
        for (set_idx, meta_line) in self.meta.borrow().iter().enumerate() {
            for (way_idx, meta_block) in meta_line.iter().enumerate() {
                let data_block = &self.data.borrow()[(set_idx * self.meta.borrow()[0].len()) + way_idx];
                let dut_data_block = &dut.data.borrow()[(set_idx * dut.meta.borrow()[0].len()) + way_idx];

                if meta_block.tag != dut.meta.borrow()[set_idx][way_idx].tag ||
                   data_block.target != dut_data_block.target {
                    log_error!(format!(
                        "BTB mismatch at Set {}, Way {}: Expected Tag: {:#08x}, Target: {:#08x}, Got Tag: {:#08x}, Target: {:#08x}",
                        set_idx, way_idx, dut.meta.borrow()[set_idx][way_idx].tag, dut_data_block.target,
                        meta_block.tag, data_block.target
                    ));
                    return Err(ProcessError::Recoverable);
                }
            }
        }

        Ok(())
    }
}

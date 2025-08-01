use comfy_table::Table;
use remu_macro::log_error;
use remu_utils::ProcessError;

use crate::cache::{CacheConfiguration, CacheTable, CacheBase, Replacement};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug)]
pub struct ICacheMeta {
    valid: bool,
    tag: u32,
}

impl ICacheMeta {
    pub fn new() -> Self {
        Self {
            valid: false, 
            tag: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ICacheData {
    pub inst: u32
}

impl ICacheData {
    pub fn new() -> Self {
        Self { inst: 0 }
    }
}

#[derive(Clone, Debug)]
pub struct ICache {
    pub table: CacheTable,

    pub base_bits: u32,
    pub block_num: u32,

    meta: Rc<RefCell<Vec<Vec<ICacheMeta>>>>,
    data: Rc<RefCell<Vec<Vec<ICacheData>>>>,

    replacement: Replacement,
}

impl CacheBase for ICache {
    type CacheData = ICacheData;

    fn new(config: CacheConfiguration) -> Self {
        let (set,way, block_num, replacement) = (
            config.set,
            config.way,
            config.block_num,
            &config.replacement,
        );

        let table = CacheTable::new(set, way, block_num);

        let base_bits = config.block_num.trailing_zeros() + 2;

        let meta = Rc::new(RefCell::new(vec![vec![ICacheMeta::new(); way as usize]; set as usize]));
        let data = Rc::new(RefCell::new(vec![vec![ICacheData::new(); block_num as usize]; (set * way) as usize]));

        Self {
            table,

            base_bits,
            block_num,

            meta,
            data,
            replacement: Replacement::new(set, way, replacement),
        }
    }

    fn base_meta_write(&mut self, set: u32, way: u32, tag: u32) {
        let meta = &mut self.meta.borrow_mut()[set as usize][way as usize];
        meta.tag = tag;
        meta.valid = true;
    }

    fn base_data_write(&mut self, set: u32, way: u32, block_num: u32, data: Self::CacheData) {
        let data_index = self.table.get_data_line_index(set, way) as u32;
        self.data.borrow_mut()[data_index as usize][block_num as usize] = data;
    }

    fn base_read(&self, set: u32, way: u32) -> Vec<ICacheData> {
        let data_index = self.table.get_data_line_index(set, way);
        self.data.borrow()[data_index].clone()
    }

    fn read(&mut self, addr: u32) -> Option<Vec<ICacheData>> {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);
        // let block_num = self.table.get_block_num(addr);

        let way = {
            self.meta.borrow()[set as usize]
                .iter()
                .position(|meta_block| meta_block.valid && (meta_block.tag == tag))
        };

        way.map(|way| {
            self.replacement.access(set, way as u32);
            self.base_read(set, way as u32)
        })
    }

    fn replace(&mut self, addr: u32, data: Vec<ICacheData>) -> Option<(u32, Vec<Self::CacheData>)>  {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);

        let way = self.replacement.way(set);
        self.replacement.access(set, way);
        self.base_meta_write(set, way, tag);

        for (block_num, data) in data.iter().enumerate() {
            self.base_data_write(set, way, block_num as u32, data.clone());
        }

        None
    }

    fn print(&self) {
        let table = Table::new();
        let mut table = table;
        table.set_header(vec!["Set", "Way", "Block", "Tag", "Inst"]);

        let meta = self.meta.borrow();
        let data = self.data.borrow();

        for (set_idx, set) in meta.iter().enumerate() {
            for (way_idx, meta_block) in set.iter().enumerate() {
                let data_index = self.table.get_data_line_index(set_idx as u32, way_idx as u32);
                let data_block = &data[data_index];
                for (block_index, data) in data_block.iter().enumerate() {
                    table.add_row(vec![
                        set_idx.to_string(),
                        way_idx.to_string(),
                        block_index.to_string(),
                        format!("{:#010x}", meta_block.tag),
                        format!("{:#010x}", data.inst),
                    ]);
                }
            }
        }

        println!("{table}");
    }

    fn test(&self, dut: &Self) -> remu_utils::ProcessResult<()> {
        for (set_idx, set) in self.meta.borrow().iter().enumerate() {
            for (way_idx, meta_block) in set.iter().enumerate() {
                let data_index = self.table.get_data_line_index(set_idx as u32, way_idx as u32);
                let data_block = &self.data.borrow()[data_index];

                if meta_block.tag != dut.meta.borrow()[set_idx][way_idx].tag {
                    log_error!(format!(
                        "ICache test failed at Set {}, Way {}: Expected Tag {:#010x}, Found {:#010x}",
                        set_idx, way_idx, meta_block.tag, dut.meta.borrow()[set_idx][way_idx].tag
                    ));
                    return Err(ProcessError::Recoverable);
                }

                let dut_data = dut.base_read(set_idx as u32, way_idx as u32,);
                if *data_block != dut_data {
                    log_error!(format!(
                        "ICache test failed at Set {}, Way {}: Expected {:?}, Found {:?}",
                        set_idx, way_idx, dut_data, data_block
                    ));
                    return Err(ProcessError::Recoverable);
                }
            }
        }

        Ok(())
    }
}

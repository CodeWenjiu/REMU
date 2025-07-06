use comfy_table::Table;

use crate::cache::{CacheConfiguration, CacheTable, CacheTrait, Replacement};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug)]
pub struct ICacheMeta {
    tag: u32,
}

impl ICacheMeta {
    pub fn new() -> Self {
        Self { tag: 0 }
    }
}

#[derive(Clone, Debug)]
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
    table: CacheTable,

    pub base_bits: u32,

    meta: Rc<RefCell<Vec<Vec<ICacheMeta>>>>,
    data: Rc<RefCell<Vec<Vec<ICacheData>>>>,

    replacement: Replacement,
}

impl CacheTrait for ICache {
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

            meta,
            data,
            replacement: Replacement::new(set, way, replacement),
        }
    }

    fn base_write(&mut self, set: u32, way: u32, block_num: u32, tag: u32, data: ICacheData) {
        let meta = &mut self.meta.borrow_mut()[set as usize][way as usize];

        let data_index = self.table.get_data_line_index(set, way);
        self.data.borrow_mut()[data_index][block_num as usize] = data;

        meta.tag = tag;
    }

    fn base_read(&self, set: u32, way: u32, block_num: u32) -> ICacheData {
        let data_index = self.table.get_data_line_index(set, way);
        self.data.borrow()[data_index][block_num as usize].clone()
    }

    fn read(&mut self, addr: u32) -> Option<ICacheData> {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);
        let block_num = self.table.get_block_num(addr);

        let way = {
            self.meta.borrow()[set as usize]
                .iter()
                .position(|meta_block| meta_block.tag == tag)
        };

        way.map(|way| {
            self.replacement.access(set, way as u32);
            self.base_read(set, way as u32, block_num)
        })
    }

    fn access(&mut self, addr: u32) {
        let set = self.table.get_set(addr);
        let way = self.replacement.way(set);
        self.replacement.access(set, way);
    }

    fn replace(&mut self, addr: u32, data: Vec<ICacheData>) {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);

        let way = self.replacement.way(set);
        self.replacement.access(set, way);

        for block_num in 0..self.base_bits {
            self.base_write(set, way, block_num, tag, data[block_num as usize].clone());
        }
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
                        meta_block.tag.to_string(),
                        format!("{:#010x}", data.inst),
                    ]);
                }
            }
        }

        println!("{table}");
    }
}

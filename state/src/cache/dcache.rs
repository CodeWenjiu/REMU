use std::{cell::RefCell, rc::Rc};

use comfy_table::Table;
use crate::cache::{CacheConfiguration, CacheTable, CacheBase, Replacement};

#[derive(Clone, Debug)]
pub struct DCacheMeta {
    valid: bool,
    dirty: bool,
    tag: u32,
}

impl DCacheMeta {
    pub fn new() -> Self {
        Self {
            valid: false,
            dirty: false,
            tag: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DCacheData {
    pub data: u32
}

impl DCacheData {
    pub fn new() -> Self {
        Self { data: 0 }
    }
}

#[derive(Clone, Debug)]
pub struct DCache {
    pub table: CacheTable,

    pub base_bits: u32,
    pub block_num: u32,

    meta: Rc<RefCell<Vec<Vec<DCacheMeta>>>>,
    data: Rc<RefCell<Vec<Vec<DCacheData>>>>,

    replacement: Replacement,
}

impl CacheBase for DCache {
    type CacheData = DCacheData;

    fn new(config: CacheConfiguration) -> Self {
        let (set, way, block_num, replacement) = (
            config.set,
            config.way,
            config.block_num,
            &config.replacement,
        );

        let table = CacheTable::new(set, way, block_num);

        let base_bits = config.block_num.trailing_zeros() + 2;

        let meta = Rc::new(RefCell::new(vec![vec![DCacheMeta::new(); way as usize]; set as usize]));
        let data = Rc::new(RefCell::new(vec![vec![DCacheData::new(); block_num as usize]; (set * way) as usize]));

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
        self.meta.borrow_mut()[set as usize][way as usize] = DCacheMeta {
            valid: true,
            dirty: false,
            tag,
        };
    }

    fn base_meta_dirt(&mut self, set: u32, way: u32) {
        self.meta.borrow_mut()[set as usize][way as usize].dirty = true;
    }

    fn base_data_write(&mut self, set: u32, way: u32, block_num: u32, data: Self::CacheData) {
        let data_index = self.table.get_data_line_index(set, way) as u32;
        self.data.borrow_mut()[data_index as usize][block_num as usize] = data;
    }

    fn base_read(&self, set: u32, way: u32) -> Vec<Self::CacheData> {
        let data_index = self.table.get_data_line_index(set, way);
        self.data.borrow()[data_index].clone()
    }

    fn read(&mut self, addr: u32) -> Option<Vec<Self::CacheData>> {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);

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

    fn write(&mut self, addr: u32, data: u32) -> Result<(), ()> {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);

        let way = self.meta.borrow()[set as usize]
                .iter()
                .position(|meta_block| meta_block.valid && (meta_block.tag == tag));


        way.map(|way| {
            self.base_data_write(set, way as u32, self.table.get_block_num(addr), DCacheData { data });
            self.replacement.access(set, way as u32);
            Ok(())
        }).unwrap_or_else(|| {
            Err(())
        })
    }

    fn replace(&mut self, addr: u32, data: Vec<Self::CacheData>) -> Option<Vec<Self::CacheData>>  {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);
        let way = self.replacement.way(set);

        let data_block = self.base_read(set, way);

        let meta_dirty = self.meta.borrow()[set as usize][way as usize].dirty;

        self.base_meta_write(set, way, tag);
        self.base_data_write(set, way, self.table.get_block_num(addr), data[0].clone());

        self.replacement.access(set, way);

        if meta_dirty {
            Some(data_block)
        } else {
            None
        }
    }

    fn print(&self) {
        let table = Table::new();
        let mut table = table;
        table.set_header(vec!["Set", "Way", "Block", "Valid", "Dirty", "Tag", "Data"]);

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
                        meta_block.valid.to_string(),
                        meta_block.dirty.to_string(),
                        format!("{:#010x}", meta_block.tag),
                        format!("{:#010x}", data.data),
                    ]);
                }
            }
        }

        println!("{table}");
    }
}
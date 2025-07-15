use std::{cell::RefCell, rc::Rc};

use comfy_table::Table;
use remu_macro::log_error;
use remu_utils::ProcessError;
use crate::{cache::{CacheBase, CacheConfiguration, CacheTable, Replacement}, mmu::Mask};

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

    fn write(&mut self, addr: u32, data: u32, mask: Mask) -> Result<(), ()> {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);

        let way = self.meta.borrow()[set as usize]
                .iter()
                .position(|meta_block| meta_block.valid && (meta_block.tag == tag));


        way.map(|way| {
            let block = self.table.get_block_num(addr);
            let mut block_data = self.base_read(set, way as u32)[block as usize].data;
            
            match mask {
                Mask::Word => {
                    self.base_data_write(set, way as u32, block, DCacheData { data: data });
                    self.replacement.access(set, way as u32);
                },
                Mask::Half => {
                    let half_offset = (addr & 0b10) as usize;
                    block_data = (block_data & !(0xFFFF << (half_offset * 8))) | ((data & 0xFFFF) << (half_offset * 8));
                    self.base_data_write(set, way as u32, block, DCacheData { data: block_data });
                    self.replacement.access(set, way as u32);
                },
                Mask::Byte => {
                    let byte_offset = (addr & 0b11) as usize;
                    block_data = (block_data & !(0xFF << (byte_offset * 8))) | ((data & 0xFF) << (byte_offset * 8));
                    self.base_data_write(set, way as u32, block, DCacheData { data: block_data });
                    self.replacement.access(set, way as u32);
                },
                _ => ()
            }

            self.base_meta_dirt(set, way as u32);
            Ok(())
        }).unwrap_or_else(|| {
            Err(())
        })
    }

    fn replace(&mut self, addr: u32, data: Vec<Self::CacheData>) -> Option<(u32, Vec<Self::CacheData>)>  {
        let set = self.table.get_set(addr);
        let tag = self.table.gat_tag(addr);
        let way = self.replacement.way(set);

        let meta_ref = self.meta.borrow();
        let meta_tag = meta_ref[set as usize][way as usize].tag;
        let meta_dirty = meta_ref[set as usize][way as usize].dirty;
        drop(meta_ref);

        let data_block = self.base_read(set, way);
        let dirty_addr = self.table.get_addr(meta_tag, set);
        let is_dirty = meta_dirty;

        self.base_meta_write(set, way, tag);
        for (block_num, block_data) in data.iter().enumerate() {
            self.base_data_write(set, way, block_num as u32, block_data.clone());
        }

        self.replacement.access(set, way);

        if is_dirty {
            Some((dirty_addr, data_block))
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

    fn test(&self, dut: &Self) -> remu_utils::ProcessResult<()> {
        for (set_idx, set) in self.meta.borrow().iter().enumerate() {
            for (way_idx, meta_block) in set.iter().enumerate() {
                let data_index = self.table.get_data_line_index(set_idx as u32, way_idx as u32);
                let data_block = &self.data.borrow()[data_index];
                let dut_meta_block = &dut.meta.borrow()[set_idx][way_idx];
                let dut_data_block = &dut.data.borrow()[data_index];

                if meta_block.tag != dut_meta_block.tag || meta_block.valid != dut_meta_block.valid || meta_block.dirty != dut_meta_block.dirty {
                    log_error!(format!(
                        "DCache test failed at Set {}, Way {}: Expected Tag {:#010x}, Valid {}, Dirty {}, Found Tag {:#010x}, Valid {}, Dirty {}",
                        set_idx, way_idx, dut_meta_block.tag, dut_meta_block.valid, dut_meta_block.dirty,
                        meta_block.tag, meta_block.valid, meta_block.dirty
                    ));
                    return Err(ProcessError::Recoverable);
                }

                if *data_block != *dut_data_block {
                    log_error!(format!(
                        "DCache test failed at Set {}, Way {}: Expected {:?}, Found {:?}",
                        set_idx, way_idx, dut_data_block, data_block
                    ));
                    return Err(ProcessError::Recoverable);
                }
            }
        }
        Ok(())
    }

    fn print_blcok(&self, addr: u32) {
        let set = self.table.get_set(addr);
        let way = self.meta.borrow()[set as usize]
            .iter()
            .position(|meta_block| meta_block.valid && (meta_block.tag == self.table.gat_tag(addr)));

        if let Some(way) = way {
            let data_index = self.table.get_data_line_index(set, way as u32);
            let data_block = &self.data.borrow()[data_index];

            let mut table = Table::new();
            for (block_index, data) in data_block.iter().enumerate() {
                table.add_row(vec![
                    set.to_string(),
                    way.to_string(),
                    block_index.to_string(),
                    format!("{:#010x}", data.data),
                ]);
            }

            println!("{table}");
        } else {
            println!("No valid block found for address {:#010x}", addr);
        }   
    }
}
use remu_macro::log_error;
use remu_utils::ProcessError;

use crate::cache::CacheTrait;

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
pub struct BtbData {
    pub target: u32,
}

impl BtbData {
    fn new() -> Self {
        Self {
            target: 0
        }
    }
}

#[derive(Debug)]
pub struct BTB {
    tag_bits: u32,
    set_bits: u32,
    way_bits: u32,
    idx_bits: u32,

    meta: Vec<Vec<BtbMeta>>,
    data: Vec<BtbData>,
}

impl BTB {
    pub fn gat_tag(&self, addr: u32) -> u32 {
        addr >> (32 - self.tag_bits)
    }

    pub fn get_set(&self, addr: u32) -> u32 {
        (addr >> self.idx_bits) & ((1 << self.set_bits) - 1)
    }

    pub fn get_idx(&self, addr: u32) -> u32 {
        addr & ((1 << self.idx_bits) - 1)
    }
}

impl CacheTrait for BTB {
    type CacheData = BtbData;

    fn new(set: u32, way: u32, block_num: u32) -> Self {
        let _ = block_num;

        assert!(set.is_power_of_two(), "set must be a power of 2");
        assert!(way.is_power_of_two(), "way must be a power of 2");
        
        let set_bits = set.trailing_zeros();
        let way_bits = way.trailing_zeros();
        let idx_bits = 2;

        BTB {
            tag_bits: 32 - (set_bits + idx_bits),
            set_bits,
            way_bits,
            idx_bits,

            meta: vec![vec![BtbMeta::new(); way as usize]; set as usize],
            data: vec![BtbData::new(); (set * way) as usize], // BTB should not have block_num
        }
    }

    fn base_write(&mut self, set: u32, way: u32, block_num: u32, tag: u32, data: BtbData) {
        let _ = block_num;

        let meta = &mut self.meta[set as usize][way as usize];

        let data_index = (set << self.way_bits) + way;
        let data_block = &mut self.data[data_index as usize];

        // Update the metadata
        meta.tag = tag;

        *data_block = data; 
    }

    fn base_read(&self, set: u32, way: u32, block_num: u32) -> &BtbData {
        let _ = block_num;
        let data_index = (set << self.way_bits) + way;
        let data_block = &self.data[data_index as usize];

        &data_block
    }

    fn read(&self, addr: u32) -> Option<&BtbData> {
        let set = self.get_set(addr);
        let meta_line = &self.meta[set as usize];

        meta_line
            .iter()
            .position(|meta_block| meta_block.tag == self.gat_tag(addr))
            .map(|way| {
                self.base_read(set, way as u32, 0)
            })
    }

    fn replace(&mut self, addr: u32, data: BtbData) {
        let set = self.get_set(addr);
        let tag = self.gat_tag(addr);

        // need to implement an way replacement algorithm

        let way = 0;
        let block_num = 0;
        self.base_write(set, way, block_num, tag, data);
    }

    fn print(&self) {
        for (set_idx, meta_line) in self.meta.iter().enumerate() {
            print!("Set {}:\t", set_idx);
            for (way_idx, meta_block) in meta_line.iter().enumerate() {
                let data_block = &self.data[(set_idx * self.meta[0].len()) + way_idx];
                println!("Way {}:\t Tag: {:#08x}, \tData: {:#08x}", way_idx, meta_block.tag, data_block.target);
            }
        }
    }

    fn test(&self, dut: &Self) -> remu_utils::ProcessResult<()> {
        for (set_idx, meta_line) in self.meta.iter().enumerate() {
            for (way_idx, meta_block) in meta_line.iter().enumerate() {
                let data_block = &self.data[(set_idx * self.meta[0].len()) + way_idx];
                let dut_data_block = &dut.data[(set_idx * dut.meta[0].len()) + way_idx];

                if meta_block.tag != dut.meta[set_idx][way_idx].tag ||
                   data_block.target != dut_data_block.target {
                    log_error!(format!(
                        "BTB mismatch at Set {}, Way {}: Expected Tag: {:#08x}, Target: {:#08x}, Got Tag: {:#08x}, Target: {:#08x}",
                        set_idx, way_idx, dut.meta[set_idx][way_idx].tag, dut_data_block.target,
                        meta_block.tag, data_block.target
                    ));
                    return Err(ProcessError::Recoverable);
                }
            }
        }

        Ok(())
    }
}

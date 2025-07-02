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
    idx_bits: u32,

    meta: Vec<Vec<BtbMeta>>,
    data: Vec<Vec<BtbData>>,
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
        assert!(set.is_power_of_two(), "set must be a power of 2");
        assert!(way.is_power_of_two(), "way must be a power of 2");
        assert!(block_num.is_power_of_two(), "block_num must be a power of 2");

        let set_bits = set.trailing_zeros();
        let base_idx = 2;
        let idx_bits = block_num.trailing_zeros() + base_idx;

        BTB {
            tag_bits: 32 - (set_bits + idx_bits),
            set_bits,
            idx_bits,

            meta: vec![vec![BtbMeta::new(); way as usize]; set as usize],
            data: vec![vec![BtbData::new(); block_num as usize]; (set * way) as usize],
        }
    }

    fn base_write(&mut self, set: u32, way: u32, block_num: u32, data: BtbData) {
        let meta = &mut self.meta[set as usize][way as usize];
        let data_block = &mut self.data[(set * way) as usize];

        // Update the metadata
        meta.tag = data.target >> (32 - self.tag_bits);

        data_block[block_num as usize] = data; 
    }

    fn base_read(&self, set: u32, way: u32, block_num: u32) -> &BtbData {
        let data_block = &self.data[(set * way) as usize];

        &data_block[block_num as usize]
    }

    fn read(&self, addr: u32) -> Option<&BtbData> {
        let set = self.get_set(addr);
        let meta_line = &self.meta[set as usize];

        meta_line
            .iter()
            .position(|meta_block| meta_block.tag == self.gat_tag(addr))
            .map(|way| {
                let idx = self.get_idx(addr);
                let block_num = idx >> 2;
                self.base_read(set, way as u32, block_num)
            })
    }

    fn replace(&mut self, addr: u32, data: BtbData) {
        let set = self.get_set(addr);

        // need to implement an way replacement algorithm

        let way = 0;
        let block_num = 0;
        self.base_write(set, way, block_num, data);
    }
}

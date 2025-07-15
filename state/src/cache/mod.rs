use remu_macro::log_todo;
use remu_utils::ProcessResult;

use crate::mmu::Mask;

remu_macro::mod_flat!(icache, dcache, btb, replacement);

#[derive(Clone, Debug)]
pub struct Cache {
    pub btb: Option<BTB>,
    pub icache: Option<ICache>,
    pub dcache: Option<DCache>,
}

impl Cache {
    pub fn new() -> Self {
        Cache { 
            btb: None, 
            icache: None,
            dcache: None,
        }
    }

    pub fn init_btb(&mut self, config: CacheConfiguration) {
        self.btb = Some(BTB::new(config));
    }

    pub fn init_icache(&mut self, config: CacheConfiguration) {
        self.icache = Some(ICache::new(config));
    }

    pub fn init_dcache(&mut self, config: CacheConfiguration) {
        self.dcache = Some(DCache::new(config));
    }
}

#[derive(Debug, Clone)]
pub struct CacheConfiguration {
    pub set: u32,
    pub way: u32,
    pub block_num: u32,
    pub replacement: String,
}

#[derive(Clone, Debug)]
pub struct CacheTable {
    tag_bits: u32,
    set_bits: u32,
    way_bits: u32,
    block_bits: u32,
    idx_bits: u32,
}

impl CacheTable {
    pub fn new(set: u32, way: u32, block_num: u32) -> Self {
        assert!(set.is_power_of_two() && way.is_power_of_two() && block_num.is_power_of_two(),
            "Set, way, and block_num must be powers of two");

        let set_bits = set.trailing_zeros();
        let way_bits = way.trailing_zeros();
        let block_bits = block_num.trailing_zeros();
        let idx_bits = 2; // Assuming a fixed index size for simplicity

        Self {
            tag_bits: 32 - (set_bits + block_bits + idx_bits),
            set_bits,
            way_bits,
            block_bits,
            idx_bits,
        }
    }

    pub fn gat_tag(&self, addr: u32) -> u32 {
        addr >> (32 - self.tag_bits)
    }

    pub fn get_set(&self, addr: u32) -> u32 {
        (addr >> (self.idx_bits + self.block_bits)) & ((1 << self.set_bits) - 1)
    }

    pub fn get_block_num(&self, addr: u32) -> u32 {
        (addr >> self.idx_bits) & ((1 << self.block_bits) - 1)
    }

    pub fn get_data_line_index(&self, set: u32, way: u32) -> usize {
        ((set << self.way_bits) + way) as usize
    }
}

#[derive(Clone, Debug)]
pub enum Replacement {
    LRU(LRU),
}

impl Replacement {
    pub fn new(set: u32, way: u32, replacement: &str) -> Self {
        match replacement {
            "lru" => Replacement::LRU(LRU::new(set, way)),
            _ => {
                panic!("Unsupported replacement policy: {}", replacement);
            }
        }
    }

    pub fn way(&self, set: u32) -> u32 {
        match self {
            Replacement::LRU(lru) => lru.way(set),
        }
    }

    pub fn access(&mut self, set: u32, way: u32) {
        match self {
            Replacement::LRU(lru) => lru.access(set, way),
        }
    }
}

pub trait CacheBase {
    type CacheData;

    fn new(config: CacheConfiguration) -> Self;
    fn base_meta_write(&mut self, set: u32, way: u32, tag: u32);
    fn base_meta_dirt(&mut self, set: u32, way: u32) {
        let _ = (set, way);
        log_todo!();
    }
    fn base_data_write(&mut self, set: u32, way: u32, block_num: u32, data: Self::CacheData);
    fn base_read(&self, set: u32, way: u32) -> Vec<Self::CacheData>;

    fn read(&mut self, addr: u32) -> Option<Vec<Self::CacheData>>;
    fn write(&mut self, addr: u32, data: u32, mask: Mask) -> Result<(), ()> {
        let _ = (addr, data, mask);
        log_todo!();
        Err(())
    }
    fn replace(&mut self, addr: u32, data: Vec<Self::CacheData>) -> Option<Vec<Self::CacheData>>;

    fn print(&self) {
        log_todo!();
    }

    fn test(&self, dut: &Self) -> ProcessResult<()> {
        let _ = dut;
        log_todo!();
        Ok(())
    }
}

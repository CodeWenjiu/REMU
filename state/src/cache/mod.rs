use std::{cell::RefCell, rc::Rc};

use remu_macro::log_todo;
use remu_utils::ProcessResult;

remu_macro::mod_flat!(btb, replacement);

#[derive(Clone, Debug)]
pub struct Cache {
    pub btb: Option<Rc<RefCell<BTB>>>,
}

impl Cache {
    pub fn new() -> Self {
        Cache { btb: None }
    }

    pub fn init_btb(&mut self, set: u32, way: u32, block_num: u32, replacement: &str) {
        self.btb = Some(Rc::new(RefCell::new(BTB::new(set, way, block_num, replacement))));
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

pub trait CacheTrait {
    type CacheData;

    fn new(set: u32, way: u32, block_num: u32, replacement: &str) -> Self;

    fn base_write(&mut self, set: u32, way: u32, block_num: u32, tag: u32, data: Self::CacheData);
    fn base_read(&self, set: u32, way: u32, block_num: u32) -> &Self::CacheData;

    fn read(&mut self, addr: u32) -> Option<&Self::CacheData>;
    fn replace(&mut self, addr: u32, data: Self::CacheData);

    fn print(&self) {
        log_todo!();
    }

    fn test(&self, dut: &Self) -> ProcessResult<()> {
        let _ = dut;
        log_todo!();
        Ok(())
    }
}

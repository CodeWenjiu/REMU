use std::{cell::RefCell, rc::Rc};

remu_macro::mod_flat!(btb);

#[derive(Clone, Debug)]
pub struct Cache {
    pub btb: Option<Rc<RefCell<BTB>>>,
}

impl Cache {
    pub fn new() -> Self {
        Cache { btb: None }
    }

    pub fn init_btb(&mut self, set: u32, way: u32, block_num: u32) {
        self.btb = Some(Rc::new(RefCell::new(BTB::new(set, way, block_num))));
    }
}

pub trait CacheTrait {
    type CacheData;

    fn new(set: u32, way: u32, block_num: u32) -> Self;

    fn base_write(&mut self, set: u32, way: u32, block_num: u32, data: Self::CacheData);
    fn base_read(&self, set: u32, way: u32, block_num: u32) -> &Self::CacheData;

    fn read(&self, addr: u32) -> Option<&Self::CacheData>;
    fn replace(&mut self, addr: u32, data: Self::CacheData);
}

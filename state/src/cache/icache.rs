// use crate::cache::{CacheTable, CacheTrait, Replacement};


// #[derive(Clone, Debug)]
// pub struct ICacheMeta {
//     tag: u32,
// }

// impl ICacheMeta {
//     pub fn new() -> Self {
//         Self { tag: 0 }
//     }
// }

// #[derive(Clone, Debug)]
// pub struct ICacheData {
//     pub inst: u32
// }

// impl ICacheData {
//     pub fn new() -> Self {
//         Self { inst: 0 }
//     }
// }

// #[derive(Debug)]
// pub struct ICache {
//     table: CacheTable,

//     meta: Vec<Vec<ICacheMeta>>,
//     data: Vec<Vec<ICacheData>>,

//     replacement: Replacement,
// }

// impl CacheTrait for ICache {
//     type CacheData = ICacheData;

//     fn new(set: u32, way: u32, block_num: u32, replacement: &str) -> Self {
//         let table = CacheTable::new(set, way, block_num);

//         let meta = vec![vec![ICacheMeta::new(); way as usize]; set as usize];
//         let data = vec![vec![ICacheData::new(); block_num as usize]; (set * way) as usize];

//         Self {
//             table,
//             meta,
//             data,
//             replacement: Replacement::new(set, way, replacement),
//         }
//     }

//     fn base_write(&mut self, set: u32, way: u32, block_num: u32, tag: u32, data: ICacheData) {
//         let meta = &mut self.meta[set as usize][way as usize];

//         let data_index = self.table.get_data_line_index(set, way);
//         self.data[data_index][block_num as usize] = data;

//         meta.tag = tag;
//     }

//     fn base_read(&self, set: u32, way: u32, block_num: u32) -> &ICacheData {
//         let data_index = self.table.get_data_line_index(set, way);
//         &self.data[data_index][block_num as usize]
//     }

//     fn read(&mut self, addr: u32) -> Option<&ICacheData> {
//         let set = self.table.get_set(addr);
//         let meta_line = &self.meta[set as usize];

//         meta_line
//             .iter()
//             .position(|meta_block| meta_block.tag == self.table.gat_tag(addr))
//             .map(|way| {
//                 self.replacement.access(set, way as u32);
//                 self.base_read(set, way as u32, 0)
//             })
//     }

//     fn replace(&mut self, addr: u32, data: ICacheData) {
//         let set = self.table.get_set(addr);
//         let tag = self.table.gat_tag(addr);

//         let way = self.replacement.way(set);
//         self.replacement.access(set, way);

//         let block_num = self.table.get_block_num(addr);
//         self.base_write(set, way, block_num, tag, data);
//     }
// }

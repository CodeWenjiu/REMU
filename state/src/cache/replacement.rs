pub trait ReplacementTrait {
    fn new(set: u32, way: u32) -> Self;
    fn way(&self, set: u32) -> u32;
    fn access(&mut self, set: u32, way: u32);
}

#[derive(Clone, Debug)]
struct LruSetQueue {
    queue: Vec<u32>,
}

impl LruSetQueue {
    fn new(way: u32) -> Self {
        Self {
            queue: (0..way).collect(),
        }
    }

    fn access(&mut self, way: u32) {
        if let Some(pos) = self.queue.iter().position(|&x| x == way) {
            let accessed_way = self.queue.remove(pos);
            self.queue.push(accessed_way);
        }
    }

    fn way_to_replace(&self) -> u32 {
        *self.queue.first().unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct LRU {
    sets: Vec<LruSetQueue>,
}

impl ReplacementTrait for LRU {
    fn new(set: u32, way: u32) -> Self {
        let sets = (0..set).map(|_| LruSetQueue::new(way)).collect();
        Self { sets }
    }

    fn way(&self, set: u32) -> u32 {
        self.sets[set as usize].way_to_replace()
    }

    fn access(&mut self, set: u32, way: u32) {
        self.sets[set as usize].access(way);
    }
}

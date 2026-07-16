//! A* search for the 15-puzzle (4x4).

use crate::bench::{Bench, Size};
use alloc::vec::Vec;
use core::fmt::Write;

const N: usize = 4;
const N2: usize = N * N;

type Grid = [u8; N2];

const PUZZLES: [Grid; 4] = [
    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0, 11, 13, 14, 15, 12], // S
    [1, 2, 3, 4, 5, 6, 7, 8, 12, 0, 14, 13, 11, 15, 10, 9], // M
    [0, 2, 3, 4, 9, 6, 7, 8, 5, 11, 10, 12, 1, 15, 13, 14], // L
    [2, 6, 8, 0, 9, 15, 4, 12, 5, 13, 11, 14, 1, 7, 3, 10], // H
];

#[derive(Clone)]
struct Puzzle {
    grid: Grid,
    zero_i: u8,
    zero_j: u8,
    manhattan: i16,
    hash_val: u32,
}

impl Puzzle {
    fn from_array(arr: &Grid) -> Self {
        let mut p = Puzzle {
            grid: *arr,
            zero_i: 0,
            zero_j: 0,
            manhattan: 0,
            hash_val: 0,
        };
        let mut m = 0i16;
        for i in 0..N {
            for j in 0..N {
                let v = p.grid[i * N + j];
                if v == 0 {
                    p.zero_i = i as u8;
                    p.zero_j = j as u8;
                } else {
                    let t = (v - 1) as usize;
                    m += abs_diff(i, t / N) as i16 + abs_diff(j, t % N) as i16;
                }
            }
        }
        p.manhattan = m;
        p.hash_val = p.compute_hash();
        p
    }
    fn solution() -> Self {
        let mut a = [0u8; N2];
        for i in 0..N2 - 1 {
            a[i] = (i + 1) as u8;
        }
        Self::from_array(&a)
    }
    fn lower_bound(&self) -> i32 {
        self.manhattan as i32
    }
    fn hash(&self) -> u32 {
        self.hash_val
    }
    fn compute_hash(&self) -> u32 {
        let mut h = 0u32;
        for i in 0..N {
            for j in 0..N {
                h = h
                    .wrapping_mul(1973)
                    .wrapping_add(self.grid[i * N + j] as u32);
            }
        }
        h
    }
    fn up_possible(&self) -> bool {
        self.zero_i != N as u8 - 1
    }
    fn down_possible(&self) -> bool {
        self.zero_i != 0
    }
    fn left_possible(&self) -> bool {
        self.zero_j != N as u8 - 1
    }
    fn right_possible(&self) -> bool {
        self.zero_j != 0
    }
    fn tile_up(&self) -> Self {
        let mut r = self.clone();
        let zi = self.zero_i as usize;
        let zj = self.zero_j as usize;
        let t = self.grid[(zi + 1) * N + zj] as usize;
        r.manhattan += abs_diff((t - 1) / N, zi) as i16 - abs_diff((t - 1) / N, zi + 1) as i16;
        r.grid[zi * N + zj] = self.grid[(zi + 1) * N + zj];
        r.zero_i += 1;
        r.grid[r.zero_i as usize * N + zj] = 0;
        r.hash_val = r.compute_hash();
        r
    }
    fn tile_down(&self) -> Self {
        let mut r = self.clone();
        let zi = self.zero_i as usize;
        let zj = self.zero_j as usize;
        let t = self.grid[(zi - 1) * N + zj] as usize;
        r.manhattan += abs_diff((t - 1) / N, zi) as i16 - abs_diff((t - 1) / N, zi - 1) as i16;
        r.grid[zi * N + zj] = self.grid[(zi - 1) * N + zj];
        r.zero_i -= 1;
        r.grid[r.zero_i as usize * N + zj] = 0;
        r.hash_val = r.compute_hash();
        r
    }
    fn tile_left(&self) -> Self {
        let mut r = self.clone();
        let zi = self.zero_i as usize;
        let zj = self.zero_j as usize;
        let t = self.grid[zi * N + zj + 1] as usize;
        r.manhattan += abs_diff((t - 1) % N, zj) as i16 - abs_diff((t - 1) % N, zj + 1) as i16;
        r.grid[zi * N + zj] = self.grid[zi * N + zj + 1];
        r.zero_j += 1;
        r.grid[zi * N + r.zero_j as usize] = 0;
        r.hash_val = r.compute_hash();
        r
    }
    fn tile_right(&self) -> Self {
        let mut r = self.clone();
        let zi = self.zero_i as usize;
        let zj = self.zero_j as usize;
        let t = self.grid[zi * N + zj - 1] as usize;
        r.manhattan += abs_diff((t - 1) % N, zj) as i16 - abs_diff((t - 1) % N, zj - 1) as i16;
        r.grid[zi * N + zj] = self.grid[zi * N + zj - 1];
        r.zero_j -= 1;
        r.grid[zi * N + r.zero_j as usize] = 0;
        r.hash_val = r.compute_hash();
        r
    }
}
impl PartialEq for Puzzle {
    fn eq(&self, o: &Self) -> bool {
        self.hash_val == o.hash_val && self.grid == o.grid
    }
}
fn abs_diff(a: usize, b: usize) -> usize {
    if a > b { a - b } else { b - a }
}

struct Step {
    puzzle: Puzzle,
    next: i32,
    heap_index: usize,
    path_length: i32,
    path_weight: i32,
}

struct UpdatableHeap {
    m: usize,
    hash_table: Vec<i32>,
    heap: Vec<i32>,
    heap_size: usize,
}

impl UpdatableHeap {
    fn new(cap: usize) -> Self {
        let m = cap.next_power_of_two();
        UpdatableHeap {
            m,
            hash_table: alloc::vec![-1i32; m],
            heap: alloc::vec![-1i32; m + 1],
            heap_size: 0,
        }
    }
    fn size(&self) -> usize {
        self.heap_size
    }
    fn pointer(&self, steps: &[Step], pz: &Puzzle) -> i32 {
        let mut ptr = self.hash_table[(pz.hash() as usize) & (self.m - 1)];
        while ptr != -1 {
            if steps[ptr as usize].puzzle == *pz {
                return ptr;
            }
            ptr = steps[ptr as usize].next;
        }
        -1
    }
    fn push(&mut self, steps: &mut Vec<Step>, pz: Puzzle, pl: i32) {
        let ptr = self.pointer(steps, &pz);
        if ptr == -1 {
            self.heap_size += 1;
            let si = steps.len() as i32;
            let hi = (pz.hash() as usize) & (self.m - 1);
            let lb = pz.lower_bound();
            steps.push(Step {
                puzzle: pz,
                next: self.hash_table[hi],
                heap_index: self.heap_size,
                path_length: pl,
                path_weight: pl + lb,
            });
            self.hash_table[hi] = si;
            self.heap[self.heap_size] = si;
            self.percolate_up(steps, self.heap_size);
        } else {
            let s = &mut steps[ptr as usize];
            let (u, hi) = {
                let w = pl + s.puzzle.lower_bound();
                if w < s.path_weight {
                    s.path_weight = w;
                    s.path_length = pl;
                    (true, s.heap_index)
                } else {
                    (false, 0)
                }
            };
            if u {
                self.percolate_up(steps, hi);
            }
        }
    }
    fn pop(&mut self, steps: &mut Vec<Step>) -> Option<Puzzle> {
        if self.heap_size == 0 {
            return None;
        }
        let top = steps[self.heap[1] as usize].puzzle.clone();
        if self.heap_size == 1 {
            self.heap_size = 0;
        } else {
            self.heap[1] = self.heap[self.heap_size];
            steps[self.heap[1] as usize].heap_index = 1;
            self.heap_size -= 1;
            self.percolate_down(steps);
        }
        Some(top)
    }
    fn length(&self, steps: &[Step], pz: &Puzzle) -> i32 {
        let p = self.pointer(steps, pz);
        if p == -1 {
            i32::MAX
        } else {
            steps[p as usize].path_length
        }
    }
    fn swap(&mut self, s: &mut [Step], i: usize, j: usize) {
        let t = self.heap[j];
        self.heap[j] = self.heap[i];
        self.heap[i] = t;
        s[self.heap[i] as usize].heap_index = i;
        s[self.heap[j] as usize].heap_index = j;
    }
    fn percolate_up(&mut self, s: &mut [Step], mut n: usize) {
        while n != 1 {
            let p = n / 2;
            if s[self.heap[p] as usize].path_weight > s[self.heap[n] as usize].path_weight {
                self.swap(s, p, n);
                n = p;
            } else {
                return;
            }
        }
    }
    fn percolate_down(&mut self, s: &mut [Step]) {
        let mut n = 1;
        loop {
            let l = 2 * n;
            let r = 2 * n + 1;
            if r > self.heap_size {
                if l == self.heap_size
                    && s[self.heap[l] as usize].path_weight < s[self.heap[n] as usize].path_weight
                {
                    self.swap(s, n, l);
                }
                return;
            }
            if s[self.heap[n] as usize].path_weight < s[self.heap[l] as usize].path_weight
                && s[self.heap[n] as usize].path_weight < s[self.heap[r] as usize].path_weight
            {
                return;
            }
            if s[self.heap[l] as usize].path_weight < s[self.heap[r] as usize].path_weight {
                self.swap(s, n, l);
                n = l;
            } else {
                self.swap(s, n, r);
                n = r;
            }
        }
    }
}

pub(crate) struct Pz15;

impl Bench for Pz15 {
    fn ref_time_usec(size: Size) -> u64 {
        match size {
            Size::Ref => 18756,
            Size::Huge => 483671,
            _ => 0,
        }
    }
    fn run<W: Write>(_w: &mut W, size: Size) -> bool {
        let (idx, maxn, checksum) = match size {
            Size::Test => (0usize, 10usize, 0x00000006),
            Size::Train => (1, 2048, 0x0000b0df),
            Size::Ref => (2, 16384, 0x00068b8c),
            Size::Huge => (3, 786432, 0x01027b4a),
        };
        let puzzle = Puzzle::from_array(&PUZZLES[idx]);
        let solution = Puzzle::solution();
        let mut heap = UpdatableHeap::new(maxn);
        let mut steps: Vec<Step> = Vec::new();
        heap.push(&mut steps, puzzle, 0);
        let mut n = 0usize;
        while heap.size() != 0 && n < maxn {
            let top = match heap.pop(&mut steps) {
                Some(t) => t,
                None => break,
            };
            n += 1;
            if top == solution {
                return (heap.length(&steps, &top) * n as i32) as u32 == checksum;
            }
            let len = heap.length(&steps, &top) + 1;
            if top.left_possible() {
                heap.push(&mut steps, top.tile_left(), len);
            }
            if top.right_possible() {
                heap.push(&mut steps, top.tile_right(), len);
            }
            if top.up_possible() {
                heap.push(&mut steps, top.tile_up(), len);
            }
            if top.down_possible() {
                heap.push(&mut steps, top.tile_down(), len);
            }
        }
        false
    }
}

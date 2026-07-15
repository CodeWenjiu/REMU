//! A* search for the 15-puzzle (4x4). Size: 0 (test). Checksum: 0x00000006.

use crate::bench::Bench;
use alloc::vec::Vec;
use core::fmt::Write;

const N: usize = 4;
const N2: usize = N * N;

/// Test puzzle from AM-Kernels (size=0)
const PUZZLE_S: [u8; N2] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0, 11, 13, 14, 15, 12];

const CHECKSUM: u32 = 0x00000006;

#[derive(Clone)]
struct Puzzle {
    grid: [u8; N2],
    zero_i: u8,
    zero_j: u8,
    manhattan: i16,
    hash_val: u32,
}

impl Puzzle {
    fn from_array(arr: &[u8; N2]) -> Self {
        let mut p = Puzzle {
            grid: *arr,
            zero_i: 0,
            zero_j: 0,
            manhattan: 0,
            hash_val: 0,
        };
        let mut manhattan = 0i16;
        for i in 0..N {
            for j in 0..N {
                let val = p.grid[i * N + j];
                if val == 0 {
                    p.zero_i = i as u8;
                    p.zero_j = j as u8;
                } else {
                    let target = (val - 1) as usize;
                    manhattan += abs_diff(i, target / N) as i16;
                    manhattan += abs_diff(j, target % N) as i16;
                }
            }
        }
        p.manhattan = manhattan;
        p.hash_val = p.compute_hash();
        p
    }

    fn solution() -> Self {
        let mut arr = [0u8; N2];
        for i in 0..N2 - 1 {
            arr[i] = (i + 1) as u8;
        }
        Puzzle::from_array(&arr)
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
        let tile = self.grid[(zi + 1) * N + zj] as usize;
        r.manhattan +=
            abs_diff((tile - 1) / N, zi) as i16 - abs_diff((tile - 1) / N, zi + 1) as i16;
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
        let tile = self.grid[(zi - 1) * N + zj] as usize;
        r.manhattan +=
            abs_diff((tile - 1) / N, zi) as i16 - abs_diff((tile - 1) / N, zi - 1) as i16;
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
        let tile = self.grid[zi * N + zj + 1] as usize;
        r.manhattan +=
            abs_diff((tile - 1) % N, zj) as i16 - abs_diff((tile - 1) % N, zj + 1) as i16;
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
        let tile = self.grid[zi * N + zj - 1] as usize;
        r.manhattan +=
            abs_diff((tile - 1) % N, zj) as i16 - abs_diff((tile - 1) % N, zj - 1) as i16;
        r.grid[zi * N + zj] = self.grid[zi * N + zj - 1];
        r.zero_j -= 1;
        r.grid[zi * N + r.zero_j as usize] = 0;
        r.hash_val = r.compute_hash();
        r
    }
}

impl PartialEq for Puzzle {
    fn eq(&self, other: &Self) -> bool {
        self.hash_val == other.hash_val && self.grid == other.grid
    }
}

fn abs_diff(a: usize, b: usize) -> usize {
    if a > b { a - b } else { b - a }
}

// ---- Updatable heap with hash table ----

struct Step {
    puzzle: Puzzle,
    next: i32,
    heap_index: usize,
    path_length: i32,
    path_weight: i32,
}

/// Binary heap with O(1) lookup via hash table.
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
        let idx = (pz.hash() as usize) & (self.m - 1);
        let mut ptr = self.hash_table[idx];
        while ptr != -1 {
            let s = &steps[ptr as usize];
            if s.puzzle == *pz {
                return ptr;
            }
            ptr = s.next;
        }
        -1
    }

    fn push(&mut self, steps: &mut Vec<Step>, pz: Puzzle, path_length: i32) {
        let ptr = self.pointer(steps, &pz);
        if ptr == -1 {
            // New node
            self.heap_size += 1;
            let step_idx = steps.len() as i32;
            let hash_idx = (pz.hash() as usize) & (self.m - 1);
            let next = self.hash_table[hash_idx];
            let weight = path_length + pz.lower_bound();

            steps.push(Step {
                puzzle: pz,
                next,
                heap_index: self.heap_size,
                path_length,
                path_weight: weight,
            });

            self.hash_table[hash_idx] = step_idx;
            self.heap[self.heap_size] = step_idx;
            self.percolate_up(steps, self.heap_size);
        } else {
            let (new_weight, heap_idx) = {
                let s = &mut steps[ptr as usize];
                let w = path_length + s.puzzle.lower_bound();
                if w < s.path_weight {
                    s.path_weight = w;
                    s.path_length = path_length;
                    (true, s.heap_index)
                } else {
                    (false, 0)
                }
            };
            if new_weight {
                self.percolate_up(steps, heap_idx);
            }
        }
    }

    fn pop(&mut self, steps: &mut Vec<Step>) -> Option<Puzzle> {
        if self.heap_size == 0 {
            return None;
        }
        let top_idx = self.heap[1] as usize;
        let top = steps[top_idx].puzzle.clone();

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
        let ptr = self.pointer(steps, pz);
        if ptr == -1 {
            i32::MAX
        } else {
            steps[ptr as usize].path_length
        }
    }

    fn swap_heap(&mut self, steps: &mut [Step], i: usize, j: usize) {
        let tmp = self.heap[j];
        self.heap[j] = self.heap[i];
        self.heap[i] = tmp;
        steps[self.heap[i] as usize].heap_index = i;
        steps[self.heap[j] as usize].heap_index = j;
    }

    fn percolate_up(&mut self, steps: &mut [Step], mut n: usize) {
        while n != 1 {
            let parent = n / 2;
            if steps[self.heap[parent] as usize].path_weight
                > steps[self.heap[n] as usize].path_weight
            {
                self.swap_heap(steps, parent, n);
                n = parent;
            } else {
                return;
            }
        }
    }

    fn percolate_down(&mut self, steps: &mut [Step]) {
        let mut n = 1usize;
        loop {
            let left = 2 * n;
            let right = 2 * n + 1;
            if right > self.heap_size {
                if left == self.heap_size
                    && steps[self.heap[left] as usize].path_weight
                        < steps[self.heap[n] as usize].path_weight
                {
                    self.swap_heap(steps, n, left);
                }
                return;
            }
            if steps[self.heap[n] as usize].path_weight
                < steps[self.heap[left] as usize].path_weight
                && steps[self.heap[n] as usize].path_weight
                    < steps[self.heap[right] as usize].path_weight
            {
                return;
            }
            if steps[self.heap[left] as usize].path_weight
                < steps[self.heap[right] as usize].path_weight
            {
                self.swap_heap(steps, n, left);
                n = left;
            } else {
                self.swap_heap(steps, n, right);
                n = right;
            }
        }
    }
}

pub(crate) struct Pz15;

impl Bench for Pz15 {
    fn run<W: Write>(_w: &mut W) -> bool {
        let puzzle = Puzzle::from_array(&PUZZLE_S);
        let maxn = 10usize;

        let mut heap = UpdatableHeap::new(maxn);
        let mut steps: Vec<Step> = Vec::new();

        let solution = Puzzle::solution();
        heap.push(&mut steps, puzzle, 0);

        let mut n = 0usize;
        let mut ans: i32 = -1;

        while heap.size() != 0 && n < maxn {
            let top = match heap.pop(&mut steps) {
                Some(t) => t,
                None => break,
            };
            n += 1;

            if top == solution {
                ans = heap.length(&steps, &top) * n as i32;
                break;
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

        ans as u32 == CHECKSUM
    }
}

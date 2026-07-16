//! Fibonacci via matrix fast exponentiation.

use crate::bench::{Bench, Size};
use alloc::vec::Vec;
use core::fmt::Write;

pub(crate) struct Fib;

impl Bench for Fib {
    fn ref_time_usec(size: Size) -> u64 {
        match size {
            Size::Ref => 396037,
            Size::Huge => 14362797,
            _ => 0,
        }
    }
    fn run<W: Write>(_w: &mut W, size: Size) -> bool {
        let (m, checksum) = match size {
            Size::Test => (2, 0x7cfeddf0),
            Size::Train => (23, 0x94ad8800),
            Size::Ref => (91, 0xebdc5f80),
            Size::Huge => (300, 0xe30a6f00),
        };
        let sz = m * m;
        let mut t: Vec<u32> = alloc::vec![0u32; sz];
        let mut ans: Vec<u32> = alloc::vec![0u32; sz];
        let mut tmp: Vec<u32> = alloc::vec![0u32; sz];

        for i in 0..m {
            for j in 0..m {
                let x = u32::from(i == m - 1 || j == i + 1);
                put(&mut t, m, i, j, x);
                put(&mut ans, m, i, j, u32::from(i == j));
            }
        }

        let mut n: u32 = 2147483603;
        while n > 0 {
            if n & 1 != 0 {
                mult(&mut tmp, &ans, &t, m);
                assign(&mut ans, &tmp, m);
            }
            mult(&mut tmp, &t, &t, m);
            assign(&mut t, &tmp, m);
            n >>= 1;
        }

        get(&ans, m, m - 1, m - 1) == checksum
    }
}

fn put(m: &mut [u32], dim: usize, i: usize, j: usize, val: u32) {
    m[i * dim + j] = val;
}
fn get(m: &[u32], dim: usize, i: usize, j: usize) -> u32 {
    m[i * dim + j]
}

fn mult(c: &mut [u32], a: &[u32], b: &[u32], m: usize) {
    for i in 0..m {
        for j in 0..m {
            let mut sum = 0u32;
            for k in 0..m {
                sum = sum.wrapping_add(get(a, m, i, k).wrapping_mul(get(b, m, k, j)));
            }
            put(c, m, i, j, sum);
        }
    }
}

fn assign(dst: &mut [u32], src: &[u32], m: usize) {
    dst[..m * m].copy_from_slice(&src[..m * m]);
}

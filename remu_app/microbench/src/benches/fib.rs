//! Fibonacci via matrix fast exponentiation.
//! f(n) = f(n-1) + ... + f(n-m), n = 2147483603.
//! Size: 2 (test). Checksum: 0x7cfeddf0.

use crate::bench::Bench;
use alloc::vec::Vec;
use core::fmt::Write;

const SIZE: usize = 2;
const N: u32 = 2147483603;
const CHECKSUM: u32 = 0x7cfeddf0;

pub(crate) struct Fib;

impl Bench for Fib {
    fn run<W: Write>(_w: &mut W) -> bool {
        let m = SIZE;
        let sz = m * m;
        let mut a: Vec<u32> = alloc::vec![0u32; sz];
        let mut t: Vec<u32> = alloc::vec![0u32; sz];
        let mut ans: Vec<u32> = alloc::vec![0u32; sz];
        let mut tmp: Vec<u32> = alloc::vec![0u32; sz];

        // Initialize A and T (companion matrix), ans = identity
        for i in 0..m {
            for j in 0..m {
                let x = u32::from(i == m - 1 || j == i + 1);
                put(&mut a, m, i, j, x);
                put(&mut t, m, i, j, x);
                put(&mut ans, m, i, j, u32::from(i == j));
            }
        }

        // Binary exponentiation: ans = T^N, then result = ans[M-1][M-1]
        let mut n = N;
        while n > 0 {
            if n & 1 != 0 {
                mult(&mut tmp, &ans, &t, m);
                assign(&mut ans, &tmp, m);
            }
            mult(&mut tmp, &t, &t, m);
            assign(&mut t, &tmp, m);
            n >>= 1;
        }

        get(&ans, m, m - 1, m - 1) == CHECKSUM
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
    let sz = m * m;
    dst[..sz].copy_from_slice(&src[..sz]);
}

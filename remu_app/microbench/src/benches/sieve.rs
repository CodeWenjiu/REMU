//! Eratosthenes sieve.

use crate::bench::{Bench, Size};
use alloc::vec::Vec;
use core::fmt::Write;

pub(crate) struct Sieve;

impl Bench for Sieve {
    fn ref_time_usec(size: Size) -> u64 {
        match size {
            Size::Ref => 139909,
            Size::Huge => 1224475,
            _ => 0,
        }
    }
    fn run<W: Write>(w: &mut W, size: Size) -> bool {
        let (n, checksum) = match size {
            Size::Test => (100, 0x00000019),
            Size::Train => (200000, 0x00004640),
            Size::Ref => (10000000, 0x000a2403),
            Size::Huge => (80000000, 0x00473fc6),
        };
        let words = n / 32 + 128;
        let mut primes: Vec<u32> = alloc::vec![0u32; words];

        let get = |i: usize, p: &[u32]| (p[i >> 5] >> (i & 31)) & 1;
        let clear = |i: usize, p: &mut [u32]| {
            p[i >> 5] &= !(1u32 << (i & 31));
        };

        for w in primes.iter_mut() {
            *w = 0xffffffff;
        }

        let mut i = 2;
        while i * i <= n {
            if get(i, &primes) != 0 {
                let mut j = i + i;
                while j <= n {
                    clear(j, &mut primes);
                    j += i;
                }
            }
            i += 1;
        }

        let mut ans = 0u32;
        for i in 2..=n {
            if get(i, &primes) != 0 {
                ans += 1;
            }
        }

        if ans != checksum {
            let _ = writeln!(w, "sieve ans={} expected={}", ans, checksum);
        }
        ans == checksum
    }
}

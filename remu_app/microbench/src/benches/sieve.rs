//! Eratosthenes sieve. Size: 100 (test). Checksum: 0x00000019.

use crate::bench::Bench;
use core::fmt::Write;

const SIZE: usize = 100;
const PRIME_WORDS: usize = SIZE / 32 + 128;
const CHECKSUM: u32 = 0x00000019;

static mut PRIMES: [u32; PRIME_WORDS] = [0u32; PRIME_WORDS];

pub(crate) struct Sieve;

impl Bench for Sieve {
    fn run<W: Write>(_w: &mut W) -> bool {
        let primes = unsafe { &mut *(&raw mut PRIMES) };

        let get = |n: usize, p: &[u32]| (p[n >> 5] >> (n & 31)) & 1;
        let clear = |n: usize, p: &mut [u32]| {
            p[n >> 5] &= !(1u32 << (n & 31));
        };

        for i in 0..=SIZE / 32 {
            primes[i] = 0xffffffff;
        }

        for i in 1..=SIZE {
            if get(i, primes) == 0 {
                return false;
            }
        }

        let mut i = 2;
        while i * i <= SIZE {
            if get(i, primes) != 0 {
                let mut j = i + i;
                while j <= SIZE {
                    clear(j, primes);
                    j += i;
                }
            }
            i += 1;
        }

        let mut ans = 0u32;
        for i in 2..=SIZE {
            if get(i, primes) != 0 {
                ans += 1;
            }
        }

        ans == CHECKSUM
    }
}

//! DEFLATE compression benchmark (replaces AM-Kernels lzip/quicklz).
//! Size: 128 (test). Checksum computed from miniz_oxide deflate output.
//!
//! Unlike the C version which uses QuickLZ (algorithm-specific checksum),
//! this uses standard DEFLATE via miniz_oxide — a real-world compression
//! workload that exercises the same CPU patterns.

use crate::bench::Bench;
use alloc::vec::Vec;
use core::fmt::Write;

const SIZE: usize = 128;
const CHECKSUM: u32 = 0xd4a742e2;

pub(crate) struct Lzip;

impl Bench for Lzip {
    fn run<W: Write>(w: &mut W) -> bool {
        let n = SIZE;

        // Generate random data
        let mut seed = 1u32;
        let mut input: Vec<u8> = alloc::vec![0u8; n];
        for i in 0..n {
            input[i] = rand15(&mut seed) as u8;
        }

        // Compress with DEFLATE
        let compressed = miniz_oxide::deflate::compress_to_vec(&input, 6);

        // Checksum over compressed output
        let cs = crate::bench::checksum(compressed.as_ptr(), unsafe {
            compressed.as_ptr().add(compressed.len())
        });
        if cs != CHECKSUM {
            let _ = writeln!(
                w,
                "lzip cs=0x{:08x} expected=0x{:08x} (raw={}, compressed={})",
                cs,
                CHECKSUM,
                n,
                compressed.len()
            );
        }
        cs == CHECKSUM
    }
}

fn rand15(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    (*seed >> 16) & 0x7fff
}

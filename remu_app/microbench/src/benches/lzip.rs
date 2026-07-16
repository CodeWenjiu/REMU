//! DEFLATE compression benchmark (replaces AM-Kernels lzip/quicklz).

use crate::bench::{Bench, Size};
use alloc::vec::Vec;
use core::fmt::Write;

pub(crate) struct Lzip;

impl Bench for Lzip {
    fn ref_time_usec(size: Size) -> u64 {
        match size {
            Size::Ref => 131969,
            Size::Huge => 3670802,
            _ => 0,
        }
    }
    fn run<W: Write>(w: &mut W, size: Size) -> bool {
        let n = match size {
            Size::Test => 128,
            Size::Train => 50000,
            Size::Ref => 1048576,
            Size::Huge => 31457280,
        };
        let checksum = match size {
            Size::Test => 0xd4a742e2,
            Size::Train => 0xc27a6c22,
            Size::Ref => 0xd579ed17,
            Size::Huge => 0x594b5d40,
        };
        let mut seed = 1u32;
        let mut input: Vec<u8> = alloc::vec![0u8; n];
        for i in 0..n {
            input[i] = rand15(&mut seed) as u8;
        }
        let compressed = miniz_oxide::deflate::compress_to_vec(&input, 6);
        let cs = crate::bench::checksum(compressed.as_ptr(), unsafe {
            compressed.as_ptr().add(compressed.len())
        });
        if cs != checksum {
            let _ = writeln!(
                w,
                "lzip cs=0x{:08x} expected=0x{:08x} (raw={}, comp={})",
                cs,
                checksum,
                n,
                compressed.len()
            );
        }
        cs == checksum
    }
}

fn rand15(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    (*seed >> 16) & 0x7fff
}

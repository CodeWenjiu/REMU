//! Quick sort.

use crate::bench::{Bench, Size};
use alloc::vec::Vec;
use core::fmt::Write;

pub(crate) struct Qsort;

impl Bench for Qsort {
    fn ref_time_usec(size: Size) -> u64 {
        match size {
            Size::Ref => 5958,
            Size::Huge => 2254171,
            _ => 0,
        }
    }
    fn run<W: Write>(w: &mut W, size: Size) -> bool {
        let (n, checksum) = match size {
            Size::Test => (100, 0x08467105),
            Size::Train => (30000, 0xa3e99fe4),
            Size::Ref => (100000, 0xed8cff89),
            Size::Huge => (4000000, 0xe6178735),
        };
        let mut data: Vec<i32> = alloc::vec![0i32; n];
        let mut seed = 1u32;
        for i in 0..n {
            let a = rand(&mut seed) as i32;
            let b = rand(&mut seed) as i32;
            data[i] = (a << 16) | b;
        }
        qsort(&mut data);
        let result =
            crate::bench::checksum(data.as_ptr() as *const u8, unsafe { data.as_ptr().add(n) }
                as *const u8);
        if result != checksum {
            let _ = writeln!(w, "qsort cs=0x{:08x} expected=0x{:08x}", result, checksum);
        }
        result == checksum
    }
}

fn rand(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    (*seed >> 16) & 0x7fff
}

fn qsort(a: &mut [i32]) {
    if a.len() <= 1 {
        return;
    }
    let mut pivot = 0;
    for j in 1..a.len() {
        if a[j] < a[0] {
            pivot += 1;
            a.swap(pivot, j);
        }
    }
    a.swap(0, pivot);
    qsort(&mut a[..pivot]);
    qsort(&mut a[pivot + 1..]);
}

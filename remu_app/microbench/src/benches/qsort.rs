//! Quick sort. Size: 100 (test). Checksum: 0x08467105.
//! Larger sizes need heap; use test variant for no_std microbench.

use crate::bench::Bench;
use core::fmt::Write;

const SIZE: usize = 100;
const CHECKSUM: u32 = 0x08467105;

pub(crate) struct Qsort;

impl Bench for Qsort {
    fn run<W: Write>(w: &mut W) -> bool {
        let mut data = [0i32; SIZE];
        let mut seed = 1u32;
        for i in 0..SIZE {
            let a = rand(&mut seed) as i32;
            let b = rand(&mut seed) as i32;
            data[i] = (a << 16) | b;
        }
        qsort(&mut data);
        let result =
            crate::bench::checksum(
                data.as_ptr() as *const u8,
                unsafe { data.as_ptr().add(SIZE) } as *const u8,
            );
        if result != CHECKSUM {
            let _ = writeln!(w, "qsort cs=0x{:08x} expected=0x{:08x}", result, CHECKSUM);
        }
        result == CHECKSUM
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

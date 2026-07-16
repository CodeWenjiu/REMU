//! Suffix array via Skew algorithm.

use crate::bench::{Bench, Size};
use alloc::vec::Vec;
use core::fmt::Write;

pub(crate) struct Ssort;

impl Bench for Ssort {
    fn ref_time_usec(size: Size) -> u64 {
        match size {
            Size::Ref => 5138,
            Size::Huge => 1392463,
            _ => 0,
        }
    }
    fn run<W: Write>(w: &mut W, size: Size) -> bool {
        let (n, checksum) = match size {
            Size::Test => (100, 0x4c555e09),
            Size::Train => (10000, 0x0db7909b),
            Size::Ref => (100000, 0x4f0ab431),
            Size::Huge => (3000000, 0xeddbd9b6),
        };
        let k = 26;
        let mut s: Vec<i32> = alloc::vec![0i32; n + 10];
        let mut sa: Vec<i32> = alloc::vec![0i32; n + 10];
        let mut seed = 1u32;
        for i in 0..n {
            s[i] = (rand15(&mut seed) % 26) as i32;
        }
        s[n] = 0;
        s[n + 1] = 0;
        s[n + 2] = 0;
        suffix_array(&mut s, &mut sa, n as i32, k);
        let cs = crate::bench::checksum(sa.as_ptr() as *const u8, unsafe { sa.as_ptr().add(n) }
            as *const u8);
        if cs != checksum {
            let _ = writeln!(w, "ssort cs=0x{:08x} expected=0x{:08x}", cs, checksum);
        }
        cs == checksum
    }
}

fn leq2(a1: i32, a2: i32, b1: i32, b2: i32) -> bool {
    a1 < b1 || (a1 == b1 && a2 <= b2)
}
fn leq3(a1: i32, a2: i32, a3: i32, b1: i32, b2: i32, b3: i32) -> bool {
    a1 < b1 || (a1 == b1 && leq2(a2, a3, b2, b3))
}

fn radix_pass(a: &[i32], b: &mut [i32], r: &[i32], n: usize, k: i32) {
    let mut c: Vec<i32> = alloc::vec![0i32; k as usize + 1];
    for i in 0..n {
        c[r[a[i] as usize] as usize] += 1;
    }
    let mut sum = 0i32;
    for i in 0..=k as usize {
        let t = c[i];
        c[i] = sum;
        sum += t;
    }
    for i in 0..n {
        let idx = r[a[i] as usize] as usize;
        b[c[idx] as usize] = a[i];
        c[idx] += 1;
    }
}

fn suffix_array(s: &mut [i32], sa: &mut [i32], n: i32, k: i32) {
    let nu = n as usize;
    let n0 = (nu + 2) / 3;
    let n1 = (nu + 1) / 3;
    let n2 = nu / 3;
    let n02 = n0 + n2;
    let mut s12: Vec<i32> = alloc::vec![0i32; n02 + 3];
    let mut sa12: Vec<i32> = alloc::vec![0i32; n02 + 3];
    let mut s0: Vec<i32> = alloc::vec![0i32; n0];
    let mut sa0: Vec<i32> = alloc::vec![0i32; n0];

    let n0_n1 = n0 as i32 - n1 as i32;
    let mut j = 0;
    for i in 0..nu + (if n0_n1 > 0 { n0_n1 as usize } else { 0 }) {
        if i % 3 != 0 {
            s12[j] = i as i32;
            j += 1;
        }
    }

    radix_pass(&s12, &mut sa12, &s[2..], n02, k);
    radix_pass(&sa12, &mut s12, &s[1..], n02, k);
    radix_pass(&s12, &mut sa12, s, n02, k);

    let mut name = 0i32;
    let mut c0 = -1i32;
    let mut c1 = -1i32;
    let mut c2 = -1i32;
    for i in 0..n02 {
        let idx = sa12[i] as usize;
        if s[idx] != c0 || s[idx + 1] != c1 || s[idx + 2] != c2 {
            name += 1;
            c0 = s[idx];
            c1 = s[idx + 1];
            c2 = s[idx + 2];
        }
        if sa12[i] % 3 == 1 {
            s12[(sa12[i] / 3) as usize] = name;
        } else {
            s12[(sa12[i] / 3) as usize + n0] = name;
        }
    }

    if name < n02 as i32 {
        suffix_array(&mut s12, &mut sa12, n02 as i32, name);
        for i in 0..n02 {
            s12[sa12[i] as usize] = i as i32 + 1;
        }
    } else {
        for i in 0..n02 {
            sa12[(s12[i] - 1) as usize] = i as i32;
        }
    }

    j = 0;
    for i in 0..n02 {
        if sa12[i] < n0 as i32 {
            s0[j] = 3 * sa12[i];
            j += 1;
        }
    }
    radix_pass(&s0, &mut sa0, s, n0, k);

    let mut p = 0;
    let mut t = if n0 >= n1 { n0 - n1 } else { 0 };
    for k in 0..nu {
        let get_i = |tv: usize| -> i32 {
            let v = sa12[tv];
            if v < n0 as i32 {
                v * 3 + 1
            } else {
                (v - n0 as i32) * 3 + 2
            }
        };
        let i = get_i(t) as usize;
        let j2 = sa0[p] as usize;
        let smaller = if sa12[t] < n0 as i32 {
            leq2(
                s[i],
                s12[(sa12[t] + n0 as i32) as usize],
                s[j2],
                s12[j2 / 3],
            )
        } else {
            leq3(
                s[i],
                s[i + 1],
                s12[(sa12[t] - n0 as i32 + 1) as usize],
                s[j2],
                s[j2 + 1],
                s12[j2 / 3 + n0],
            )
        };
        if smaller {
            sa[k] = i as i32;
            t += 1;
            if t == n02 {
                for k2 in (k + 1)..nu {
                    sa[k2] = sa0[p];
                    p += 1;
                }
                break;
            }
        } else {
            sa[k] = j2 as i32;
            p += 1;
            if p == n0 {
                for k2 in (k + 1)..nu {
                    sa[k2] = get_i(t);
                    t += 1;
                }
                break;
            }
        }
    }
}

fn rand15(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    (*seed >> 16) & 0x7fff
}

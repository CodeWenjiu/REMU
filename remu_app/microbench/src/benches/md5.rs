//! MD5 digest of a random string.

use crate::bench::{Bench, Size};
use alloc::vec::Vec;
use core::fmt::Write;

pub(crate) struct Md5;

impl Bench for Md5 {
    fn ref_time_usec(size: Size) -> u64 {
        match size {
            Size::Ref => 30350,
            Size::Huge => 1976871,
            _ => 0,
        }
    }
    fn run<W: Write>(w: &mut W, size: Size) -> bool {
        let (n, checksum) = match size {
            Size::Test => (100, 0xf902f28f),
            Size::Train => (200000, 0xd4f9bc6d),
            Size::Ref => (10000000, 0x27286a42),
            Size::Huge => (64000000, 0x41ab4d60),
        };
        let buf_size = ((n + 72) / 64) * 64;
        let mut msg: Vec<u8> = alloc::vec![0u8; buf_size];
        let mut digest = [0u8; 16];

        let mut seed = 1u32;
        for i in 0..n {
            msg[i] = rand15(&mut seed) as u8;
        }

        md5(&mut msg, n, &mut digest);

        let cs = crate::bench::checksum(digest.as_ptr(), unsafe { digest.as_ptr().add(16) });
        if cs != checksum {
            let _ = writeln!(w, "md5 cs=0x{:08x} expected=0x{:08x}", cs, checksum);
        }
        cs == checksum
    }
}

fn rand15(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    (*seed >> 16) & 0x7fff
}

fn left_rotate(x: u32, c: u32) -> u32 {
    (x << c) | (x >> (32u32.wrapping_sub(c)))
}

fn to_bytes(val: u32, bytes: &mut [u8]) {
    bytes[0] = val as u8;
    bytes[1] = (val >> 8) as u8;
    bytes[2] = (val >> 16) as u8;
    bytes[3] = (val >> 24) as u8;
}

fn to_int32(bytes: &[u8]) -> u32 {
    bytes[0] as u32
        | ((bytes[1] as u32) << 8)
        | ((bytes[2] as u32) << 16)
        | ((bytes[3] as u32) << 24)
}

const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

const R: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

fn md5(msg: &mut [u8], initial_len: usize, digest: &mut [u8; 16]) {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xefcdab89;
    let mut h2: u32 = 0x98badcfe;
    let mut h3: u32 = 0x10325476;

    let mut new_len = initial_len + 1;
    while new_len % 64 != 56 {
        new_len += 1;
    }

    msg[initial_len] = 0x80;
    for i in (initial_len + 1)..new_len {
        msg[i] = 0;
    }

    let bits = (initial_len as u64).wrapping_mul(8);
    to_bytes(bits as u32, &mut msg[new_len..]);
    to_bytes((bits >> 32) as u32, &mut msg[(new_len + 4)..]);

    for chunk_start in (0..new_len).step_by(64) {
        let mut w = [0u32; 16];
        for i in 0..16 {
            w[i] = to_int32(&msg[(chunk_start + i * 4)..]);
        }
        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        for i in 0..64 {
            let (f, g) = if i < 16 {
                ((b & c) | ((!b) & d), i)
            } else if i < 32 {
                ((d & b) | ((!d) & c), (5 * i + 1) % 16)
            } else if i < 48 {
                (b ^ c ^ d, (3 * i + 5) % 16)
            } else {
                (c ^ (b | (!d)), (7 * i) % 16)
            };
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(left_rotate(
                a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(w[g]),
                R[i],
            ));
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
    }
    to_bytes(h0, &mut digest[0..4]);
    to_bytes(h1, &mut digest[4..8]);
    to_bytes(h2, &mut digest[8..12]);
    to_bytes(h3, &mut digest[12..16]);
}

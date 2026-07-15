//! N-Queens via bit manipulation. Size: 12 (ref).
//! Checksum: 0x00003778

use crate::bench::Bench;
use core::fmt::Write;

const SIZE: u32 = 12;
const CHECKSUM: u32 = 0x00003778;

pub(crate) struct Queen;

impl Bench for Queen {
    fn run<W: Write>(_w: &mut W) -> bool {
        let full = (1u32 << SIZE) - 1;
        let ans = dfs(0, 0, 0, full);
        ans == CHECKSUM
    }
}

fn dfs(row: u32, ld: u32, rd: u32, full: u32) -> u32 {
    if row == full {
        return 1;
    }
    let mut pos = full & (!(row | ld | rd));
    let mut ans = 0;
    while pos != 0 {
        let p = pos & pos.wrapping_neg();
        pos -= p;
        ans += dfs(row | p, (ld | p) << 1, (rd | p) >> 1, full);
    }
    ans
}

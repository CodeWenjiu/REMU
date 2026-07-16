//! N-Queens via bit manipulation.

use crate::bench::{Bench, Size};
use core::fmt::Write;

pub(crate) struct Queen;

impl Bench for Queen {
    fn ref_time_usec(size: Size) -> u64 {
        match size {
            Size::Ref => 5798,
            Size::Huge => 1308621,
            _ => 0,
        }
    }
    fn run<W: Write>(_w: &mut W, size: Size) -> bool {
        let (n, checksum) = match size {
            Size::Test => (8, 0x0000005c),
            Size::Train => (11, 0x00000a78),
            Size::Ref => (12, 0x00003778),
            Size::Huge => (15, 0x0022c710),
        };
        let full = (1u32 << n) - 1;
        dfs(0, 0, 0, full) == checksum
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

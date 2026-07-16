//! Brainf**k interpreter.

use crate::bench::{Bench, Size};
use alloc::vec::Vec;
use core::fmt::Write;

pub(crate) struct Bf;

impl Bench for Bf {
    fn ref_time_usec(size: Size) -> u64 {
        match size {
            Size::Ref => 15931,
            Size::Huge => 3578112,
            _ => 0,
        }
    }
    fn run<W: Write>(w: &mut W, size: Size) -> bool {
        let (n, checksum) = match size {
            Size::Test => (2, 0xa6f0079e),
            Size::Train => (25, 0xa88f8a65),
            Size::Ref => (180, 0x9221e2b3),
            Size::Huge => (1360, 0xdb49fbde),
        };
        let program_size = 4096;
        let stack_size = 512;
        let data_size = 4096;
        let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let code = ">>+>>>>>,[>+>>,]>+[--[+<<<-]<[<+>-]<[<[->[<<<+>>>>+<-]<<[>>+>[->]<<[<]<-]>]>>>+<[[-]<[>+<-]<]>[[>>>]+<<<-<[<<[<<<]>>+>[>>>]<-]<<[<<<]>[>>[>>>]<+<<[<<<]>-]]+<<<]+[->>>]>>]>>[.>>>]";

        let mut prog: Vec<(u16, u16)> = alloc::vec![(0, 0); program_size];
        let mut stack: Vec<u16> = Vec::with_capacity(stack_size);
        let mut data: Vec<u16> = alloc::vec![0u16; data_size];
        let mut input: Vec<u8> = alloc::vec![0u8; n + 1];
        let mut output: Vec<u8> = alloc::vec![0u8; data_size];

        let mut seed = 1u32;
        for i in 0..n {
            input[i] = alphabet[(rand15(&mut seed) as usize) % 62];
        }
        let mut input_idx = 0usize;

        if compile(code, &mut prog, &mut stack).is_err() {
            let _ = writeln!(w, "bf compile error");
            return false;
        }

        let mut noutput = 0usize;
        let mut pc = 0usize;
        let mut ptr = 0usize;
        loop {
            let (op, operand) = prog[pc];
            match op {
                0 => break,
                1 => ptr += 1,
                2 => ptr = ptr.wrapping_sub(1),
                3 => data[ptr] = data[ptr].wrapping_add(1),
                4 => data[ptr] = data[ptr].wrapping_sub(1),
                5 => {
                    output[noutput] = data[ptr] as u8;
                    noutput += 1;
                }
                6 => {
                    data[ptr] = input[input_idx] as u16;
                    input_idx += 1;
                }
                7 => {
                    if data[ptr] == 0 {
                        pc = operand as usize;
                    }
                }
                8 => {
                    if data[ptr] != 0 {
                        pc = operand as usize;
                    }
                }
                _ => {
                    let _ = writeln!(w, "bf unknown op");
                    return false;
                }
            }
            if ptr >= data_size {
                let _ = writeln!(w, "bf ptr overflow");
                return false;
            }
            pc += 1;
        }

        if noutput != n {
            let _ = writeln!(w, "bf noutput={} expected={}", noutput, n);
            return false;
        }

        let cs = crate::bench::checksum(output.as_ptr(), unsafe { output.as_ptr().add(noutput) });
        if cs != checksum {
            let _ = writeln!(w, "bf cs=0x{:08x} expected=0x{:08x}", cs, checksum);
        }
        cs == checksum
    }
}

fn compile(code: &str, prog: &mut [(u16, u16)], stack: &mut Vec<u16>) -> Result<(), ()> {
    let mut pc = 0usize;
    for c in code.bytes() {
        if pc >= 4096 {
            return Err(());
        }
        match c {
            b'>' => {
                prog[pc].0 = 1;
                pc += 1;
            }
            b'<' => {
                prog[pc].0 = 2;
                pc += 1;
            }
            b'+' => {
                prog[pc].0 = 3;
                pc += 1;
            }
            b'-' => {
                prog[pc].0 = 4;
                pc += 1;
            }
            b'.' => {
                prog[pc].0 = 5;
                pc += 1;
            }
            b',' => {
                prog[pc].0 = 6;
                pc += 1;
            }
            b'[' => {
                if stack.len() >= 512 {
                    return Err(());
                }
                stack.push(pc as u16);
                prog[pc].0 = 7;
                pc += 1;
            }
            b']' => {
                let jmp = stack.pop().ok_or(())?;
                prog[pc].0 = 8;
                prog[pc].1 = jmp;
                prog[jmp as usize].1 = pc as u16;
                pc += 1;
            }
            _ => {}
        }
    }
    if !stack.is_empty() || pc == 4096 {
        return Err(());
    }
    prog[pc].0 = 0;
    Ok(())
}

fn rand15(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    (*seed >> 16) & 0x7fff
}

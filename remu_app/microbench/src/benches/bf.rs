//! Brainf**k interpreter. Size: 2 (test). Checksum: 0xa6f0079e.

use crate::bench::Bench;
use alloc::vec::Vec;
use core::fmt::Write;

const SIZE: usize = 2;
const CHECKSUM: u32 = 0xa6f0079e;

const PROGRAM_SIZE: usize = 4096;
const STACK_SIZE: usize = 512;
const DATA_SIZE: usize = 4096;

const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

const CODE: &str = ">>+>>>>>,[>+>>,]>+[--[+<<<-]<[<+>-]<[<[->[<<<+>>>>+<-]<<[>>+>[->]<<[<]<-]>]>>>+<[[-]<[>+<-]<]>[[>>>]+<<<-<[<<[<<<]>>+>[>>>]<-]<<[<<<]>[>>[>>>]<+<<[<<<]>-]]+<<<]+[->>>]>>]>>[.>>>]";

#[derive(Clone, Copy)]
struct Instruction {
    op: u16,
    operand: u16,
}

const OP_END: u16 = 0;
const OP_INC_DP: u16 = 1;
const OP_DEC_DP: u16 = 2;
const OP_INC_VAL: u16 = 3;
const OP_DEC_VAL: u16 = 4;
const OP_OUT: u16 = 5;
const OP_IN: u16 = 6;
const OP_JMP_FWD: u16 = 7;
const OP_JMP_BCK: u16 = 8;

pub(crate) struct Bf;

impl Bench for Bf {
    fn run<W: Write>(w: &mut W) -> bool {
        let mut program: Vec<Instruction> =
            alloc::vec![Instruction { op: 0, operand: 0 }; PROGRAM_SIZE];
        let mut stack: Vec<u16> = Vec::with_capacity(STACK_SIZE);
        let mut data: Vec<u16> = alloc::vec![0u16; DATA_SIZE];
        let mut input: Vec<u8> = alloc::vec![0u8; SIZE + 1];
        let mut output: Vec<u8> = alloc::vec![0u8; DATA_SIZE];

        let mut seed = 1u32;
        for i in 0..SIZE {
            let r = rand15(&mut seed);
            input[i] = ALPHABET[(r as usize) % 62];
        }
        let mut input_idx = 0usize;

        if compile(&CODE, &mut program, &mut stack).is_err() {
            let _ = writeln!(w, "bf compile error");
            return false;
        }

        let mut noutput = 0usize;
        let mut pc = 0usize;
        let mut ptr = 0usize;
        loop {
            let inst = program[pc];
            match inst.op {
                OP_END => break,
                OP_INC_DP => ptr += 1,
                OP_DEC_DP => ptr = ptr.wrapping_sub(1),
                OP_INC_VAL => data[ptr] = data[ptr].wrapping_add(1),
                OP_DEC_VAL => data[ptr] = data[ptr].wrapping_sub(1),
                OP_OUT => {
                    output[noutput] = data[ptr] as u8;
                    noutput += 1;
                }
                OP_IN => {
                    data[ptr] = input[input_idx] as u16;
                    input_idx += 1;
                }
                OP_JMP_FWD => {
                    if data[ptr] == 0 {
                        pc = inst.operand as usize;
                    }
                }
                OP_JMP_BCK => {
                    if data[ptr] != 0 {
                        pc = inst.operand as usize;
                    }
                }
                _ => {
                    let _ = writeln!(w, "bf unknown op");
                    return false;
                }
            }
            if ptr >= DATA_SIZE {
                let _ = writeln!(w, "bf ptr overflow");
                return false;
            }
            pc += 1;
        }

        if noutput != SIZE {
            let _ = writeln!(w, "bf noutput={} expected={}", noutput, SIZE);
            return false;
        }

        let cs = crate::bench::checksum(output.as_ptr(), unsafe { output.as_ptr().add(noutput) });
        if cs != CHECKSUM {
            let _ = writeln!(w, "bf cs=0x{:08x} expected=0x{:08x}", cs, CHECKSUM);
        }
        cs == CHECKSUM
    }
}

#[derive(Debug)]
struct CompileError;

fn compile(code: &str, prog: &mut [Instruction], stack: &mut Vec<u16>) -> Result<(), CompileError> {
    let mut pc = 0usize;
    for c in code.bytes() {
        if pc >= PROGRAM_SIZE {
            return Err(CompileError);
        }
        let op = match c {
            b'>' => OP_INC_DP,
            b'<' => OP_DEC_DP,
            b'+' => OP_INC_VAL,
            b'-' => OP_DEC_VAL,
            b'.' => OP_OUT,
            b',' => OP_IN,
            b'[' => {
                if stack.len() >= STACK_SIZE {
                    return Err(CompileError);
                }
                stack.push(pc as u16);
                OP_JMP_FWD
            }
            b']' => {
                let jmp_pc = stack.pop().ok_or(CompileError)?;
                prog[pc].op = OP_JMP_BCK;
                prog[pc].operand = jmp_pc;
                prog[jmp_pc as usize].operand = pc as u16;
                pc += 1;
                continue;
            }
            _ => continue,
        };
        prog[pc].op = op;
        pc += 1;
    }
    if !stack.is_empty() || pc == PROGRAM_SIZE {
        return Err(CompileError);
    }
    prog[pc].op = OP_END;
    Ok(())
}

fn rand15(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    (*seed >> 16) & 0x7fff
}

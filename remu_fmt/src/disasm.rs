use capstone::{
    Capstone,
    arch::{self, BuildsCapstone},
};
use target_lexicon::Architecture;
use thiserror::Error;

fn format_insn(insn: &Option<&capstone::Insn>) -> String {
    match insn {
        Some(inst) => {
            let mnemonic = inst.mnemonic().unwrap_or("");
            let op_str = inst.op_str().unwrap_or("");

            if op_str.is_empty() {
                mnemonic.to_string()
            } else {
                format!("{mnemonic} {op_str}")
            }
        }
        None => "???".to_string(),
    }
}

fn guess_ascii_word(bytes: [u8; 4]) -> Option<String> {
    // Heuristic: treat as "string-like" if we have no "bad" control chars and
    // at least 3/4 bytes are printable (graphic), space, allowed whitespace, or '\0'.
    let allowed = bytes
        .iter()
        .copied()
        .filter(|b| {
            *b == 0
                || *b == b' '
                || *b == b'\t'
                || *b == b'\n'
                || *b == b'\r'
                || b.is_ascii_graphic()
        })
        .count();

    let bad_control = bytes
        .iter()
        .copied()
        .filter(|b| b.is_ascii_control() && !matches!(*b, 0 | b'\t' | b'\n' | b'\r'))
        .count();

    if bad_control != 0 || allowed < 3 {
        return None;
    }

    // Render as a quoted string, escaping special bytes like \0, \n, etc.
    let mut out = String::with_capacity(2 + 4 * 4);
    out.push('"');
    for b in bytes {
        match b {
            0 => out.push_str("\\0"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            b if b == b' ' || b.is_ascii_graphic() => out.push(b as char),
            b => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out.push('"');
    Some(out)
}

#[derive(Debug, Error)]
pub enum GuessError {
    #[error("Failed to disasm {0}")]
    DisasmError(#[from] capstone::Error),
}

pub struct ByteGuesser {
    cs: Capstone,
}

impl ByteGuesser {
    pub fn new(isa: Architecture) -> Self {
        let arch_mode = match isa {
            Architecture::Riscv32(_) => arch::riscv::ArchMode::RiscV32,
            Architecture::Riscv64(_) => arch::riscv::ArchMode::RiscV64,
            _ => unreachable!(),
        };

        let cs = Capstone::new()
            .riscv()
            .mode(arch_mode)
            .detail(true)
            .build()
            .expect("Failed to build Disassembler");

        Self { cs }
    }

    pub fn disassemble(&self, addr: u64, bytes: u32) -> Result<String, GuessError> {
        let insns = self.cs.disasm_count(&bytes.to_le_bytes(), addr, 1)?;
        Ok(format_insn(&insns.get(0)))
    }

    pub fn guess(&self, addr: u64, bytes: u32) -> String {
        // 1) Try to decode as an instruction.
        if let Ok(insns) = self.cs.disasm_count(&bytes.to_le_bytes(), addr, 1) {
            let inst = insns.get(0);
            if inst.is_some() {
                return format_insn(&inst);
            }
        }

        // 2) Fallback: does it look like an ASCII-ish string?
        if let Some(s) = guess_ascii_word(bytes.to_le_bytes()) {
            return s;
        }

        // 3) Give up.
        "???".to_string()
    }
}

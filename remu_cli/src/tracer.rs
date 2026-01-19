use remu_types::Tracer;
use tabled::{Table, Tabled, settings::Style};

use colored::Colorize;

pub struct CLITracer;

#[derive(Clone, Copy, Debug)]
pub struct HexU32(pub u32);

impl std::fmt::Display for HexU32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:08x}", self.0)
    }
}

#[derive(Tabled)]
pub struct MemTable {
    address: HexU32,
    data: HexU32,
    interpretation: String,
}

fn mem_rows_32(begin: usize, data: &[u8]) -> Vec<MemTable> {
    let mut rows = Vec::new();

    // One line displays 32-bit data, i.e., 4 bytes per row.
    // If the final chunk is shorter than 4 bytes, pad with zeros.
    for (i, chunk) in data.chunks(4).enumerate() {
        let addr = begin + i * 4;

        let mut bytes = [0u8; 4];
        bytes[..chunk.len()].copy_from_slice(chunk);

        let word = u32::from_le_bytes(bytes);

        rows.push(MemTable {
            address: HexU32(addr as u32),
            data: HexU32(word),
            interpretation: "nop".to_string(), // nop for now
        });
    }

    rows
}

impl Tracer for CLITracer {
    fn mem_print(&self, begin: usize, data: &[u8], result: Result<(), Box<dyn std::error::Error>>) {
        match result {
            Ok(_) => {
                let rows = mem_rows_32(begin, data);
                let mut table = Table::new(rows);
                table.with(Style::rounded());
                println!("{table}");
            }
            Err(err) => self.deal_error(err),
        }
    }

    fn deal_error(&self, error: Box<dyn std::error::Error>) {
        println!("{}{}", "error: ".red(), error.to_string())
    }
}

impl CLITracer {
    pub fn new() -> Self {
        CLITracer
    }
}

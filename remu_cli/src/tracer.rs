use colored::Colorize;
use remu_types::{DynDiagError, Tracer};
use tabled::{
    Table, Tabled,
    settings::{Color, Style, object::Columns},
};

pub struct CLITracer;

fn display_address(val: &u32) -> String {
    format!("0x{:08x}", val).to_string()
}

#[derive(Tabled)]
pub struct MemTable {
    #[tabled(display = "display_address")]
    address: u32,
    #[tabled(display = "display_address")]
    data: u32,
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
            address: addr as u32,
            data: word,
            interpretation: "nop".to_string(), // nop for now
        });
    }

    rows
}

impl Tracer for CLITracer {
    fn mem_print(&self, begin: usize, data: &[u8], result: Result<(), Box<dyn DynDiagError>>) {
        match result {
            Ok(_) => {
                let rows = mem_rows_32(begin, data);
                let mut table = Table::new(rows);
                table.with(Style::rounded());
                table.modify(Columns::one(0), Color::FG_YELLOW);
                table.modify(Columns::one(1), Color::FG_CYAN);
                table.modify(Columns::one(2), Color::FG_MAGENTA);
                println!("{table}");
            }
            Err(err) => self.deal_error(err),
        }
    }

    fn deal_error(&self, error: Box<dyn DynDiagError>) {
        println!("{}: {}", "error".red(), error)
    }
}

impl CLITracer {
    pub fn new() -> Self {
        CLITracer
    }
}

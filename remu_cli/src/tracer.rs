use std::ops::Range;

use colored::Colorize;
use remu_fmt::ByteGuesser;
use remu_types::{DynDiagError, Gpr, IsaSpec, Tracer};
use tabled::{
    Table, Tabled,
    settings::{Color, Style, object::Columns},
};

pub struct CLITracer {
    guesser: ByteGuesser,
}

fn display_address(val: &u32) -> String {
    format!("0x{:08x}", val).to_string()
}

fn display_bytes4(val: &String) -> String {
    val.clone()
}

fn display_data_masked(value: &u32, row: &MemTable) -> String {
    // Render like "0x????????" but replace missing bytes (outside the requested print range)
    // with spaces (2 spaces per missing byte).
    //
    // NOTE: We print MSB first to match conventional hex formatting.
    let bytes = value.to_le_bytes();
    let mut out = String::from("0x");
    for i in (0..4).rev() {
        let bit = 1u8 << i;
        if (row.byte_mask & bit) != 0 {
            out.push_str(&format!("{:02x}", bytes[i]));
        } else {
            out.push_str("  ");
        }
    }
    out
}

#[derive(Tabled)]
pub struct MemTable {
    #[tabled(display = "display_address")]
    address: u32,
    #[tabled(display("display_data_masked", self))]
    data: u32,
    #[tabled(display = "display_bytes4")]
    bytes: String,
    interpretation: String,

    #[tabled(skip)]
    byte_mask: u8,
}

fn fmt_hex(v: &u32) -> String {
    format!("0x{v:08x}")
}

#[derive(Tabled)]
pub struct RegTable {
    #[tabled()]
    register: Gpr,
    #[tabled(display = "fmt_hex")]
    data: u32,
}

impl CLITracer {
    fn mem_rows_32(&self, begin: usize, data: &[u8]) -> Vec<MemTable> {
        let mut rows = Vec::new();

        // Align rendering to 4-byte boundaries, even if `begin` is unaligned.
        //
        // We treat the printed region as [begin, begin + data.len()).
        // Rows are 4-byte aligned addresses covering that region, and each row shows up to 4 bytes.
        // Bytes outside the requested region are shown as blanks (not zeros) to avoid implying data.
        let start = begin;
        let end = begin.saturating_add(data.len());

        let aligned_start = start & !0x3;
        let aligned_end = (end + 3) & !0x3;

        let mut addr = aligned_start;
        while addr < aligned_end {
            let mut bytes_for_word = [0u8; 4];

            // byte_mask bit i means byte[i] is in-range for this row (little-endian byte index)
            let mut byte_mask: u8 = 0;

            for j in 0..4 {
                let a = addr + j;
                if a >= start && a < end {
                    let idx = a - start;
                    if let Some(b) = data.get(idx).copied() {
                        bytes_for_word[j] = b;
                        byte_mask |= 1u8 << j;
                    }
                }
            }

            // Build bytes column with blanks for out-of-range bytes.
            let mut b0 = "  ".to_string();
            let mut b1 = "  ".to_string();
            let mut b2 = "  ".to_string();
            let mut b3 = "  ".to_string();

            for j in 0..4 {
                let a = addr + j;
                if a >= start && a < end {
                    let idx = a - start;
                    if let Some(b) = data.get(idx).copied() {
                        let s = format!("{:02x}", b);
                        match j {
                            0 => b0 = s,
                            1 => b1 = s,
                            2 => b2 = s,
                            3 => b3 = s,
                            _ => {}
                        }
                    }
                }
            }

            let bytes_str = format!("[{:>2} {:>2} {:>2} {:>2}]", b0, b1, b2, b3);

            // Word is still computed as LE from the (possibly partial) bytes.
            // The display layer will mask missing bytes in the hex rendering.
            let word = u32::from_le_bytes(bytes_for_word);

            // If there are no in-range bytes for this row, skip it entirely.
            // (This should not normally occur, but keeps the output logically consistent.)
            if byte_mask == 0 {
                addr += 4;
                continue;
            }

            rows.push(MemTable {
                address: addr as u32,
                data: word,
                bytes: bytes_str,
                interpretation: self.guesser.guess(addr as u64, word),
                byte_mask,
            });

            addr += 4;
        }

        rows
    }
}

impl Tracer for CLITracer {
    fn mem_print(&self, begin: usize, data: &[u8], result: Result<(), Box<dyn DynDiagError>>) {
        match result {
            Ok(_) => {
                let rows = self.mem_rows_32(begin, data);
                let mut table = Table::new(rows);
                table.with(Style::rounded());
                table.modify(Columns::one(0), Color::FG_YELLOW);
                table.modify(Columns::one(1), Color::FG_CYAN);
                table.modify(Columns::one(2), Color::FG_BLUE);
                table.modify(Columns::one(3), Color::FG_BRIGHT_WHITE);
                println!("{table}");
            }
            Err(err) => self.deal_error(err),
        }
    }

    fn mem_show(&self, begin: usize, data: Result<remu_types::AllUsize, Box<dyn DynDiagError>>) {
        match data {
            Ok(value) => println!(
                "{}: {}",
                format!("0x{:08x}", begin).yellow(),
                format!("{}", value).blue()
            ),
            Err(err) => self.deal_error(err),
        }
    }

    fn reg_print(&self, regs: &[(Gpr, u32); 32], range: Range<usize>) {
        // `range` is a half-open index range over the regs slice (start..end).
        // Clamp to slice bounds to avoid panics and make UX nicer.
        let start = range.start.min(regs.len());
        let end = range.end.min(regs.len());

        let mut table = Table::new(regs[start..end].iter().map(|&(reg, data)| RegTable {
            register: reg,
            data,
        }));
        table.with(Style::rounded());
        table.modify(Columns::one(0), Color::FG_YELLOW);
        table.modify(Columns::one(1), Color::FG_CYAN);
        println!("{table}");
    }

    fn reg_show(&self, index: Gpr, data: u32) {
        println!(
            "index: {}, data: {}",
            format!("{}", index).yellow(),
            format!("0x{:08x}", data).blue()
        )
    }

    fn deal_error(&self, error: Box<dyn DynDiagError>) {
        println!("{}: {}", "error".red(), error)
    }
}

impl CLITracer {
    pub fn new(isa: IsaSpec) -> Self {
        CLITracer {
            guesser: ByteGuesser::new(isa.0),
        }
    }
}

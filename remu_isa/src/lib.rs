remu_macro::mod_pub!(isa);
remu_macro::mod_flat!(wordlen);

use std::fmt::Display;

/// Union type for difftest comparison — holds any RISC-V machine word size.
#[derive(Debug, Clone)]
pub enum AllUsize {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    /// Fixed-length bytes (e.g. for memory difftest)
    Bytes(Box<[u8]>),
}

impl Display for AllUsize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AllUsize::U8(v) => write!(f, "0x{:02x}", v),
            AllUsize::U16(v) => write!(f, "0x{:04x}", v),
            AllUsize::U32(v) => write!(f, "0x{:08x}", v),
            AllUsize::U64(v) => write!(f, "0x{:016x}", v),
            AllUsize::U128(v) => write!(f, "0x{:032x}", v),
            AllUsize::Bytes(v) => {
                if v.is_empty() {
                    write!(f, "(empty)")
                } else if v.len() <= 16 {
                    write!(
                        f,
                        "0x{}",
                        v.iter().map(|b| format!("{:02x}", b)).collect::<String>()
                    )
                } else {
                    write!(
                        f,
                        "0x{}.. ({} bytes)",
                        v[..8]
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<String>(),
                        v.len()
                    )
                }
            }
        }
    }
}

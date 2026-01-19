use thiserror::Error;

#[derive(Debug, clap::Subcommand)]
pub enum StateCmds {
    /// Hello Test
    Hello,

    /// Print Memory Contents
    Print {
        /// Address to start printing from (e.g. `0x1000`, `0o377`, `0b1010`, `1234`, `0d1234`)
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        start: usize,

        /// Number of bytes to print (e.g. `16`, `0x10`)
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        count: usize,
    },

    /// Set Memory Value
    Set {
        /// Address to set
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        address: usize,
        /// Value to set (e.g. `0xdead_beef` or `[0xde, 0xad, 0xbe, 0xef]` or `[0xdead, 0xbe, 0xef]`)
        #[arg(value_parser = parse_byte_vec)]
        value: Vec<Vec<u8>>,
    },
}

#[derive(Debug, Error)]
pub enum ParseLiteralError {
    #[error("invalid number '{input}': missing digits")]
    MissingDigits { input: String },

    #[error("invalid number '{input}': {source}")]
    InvalidNumber {
        input: String,
        #[source]
        source: std::num::ParseIntError,
    },

    #[error("number '{input}' out of range: {details}")]
    OutOfRange { input: String, details: String },

    #[error("invalid radix for '{input}': {radix}")]
    InvalidRadix { input: String, radix: u32 },

    #[error(
        "invalid number '{input}': too many digits for u128-backed byte parsing ({bytes_requested} bytes requested)"
    )]
    TooManyBytes {
        input: String,
        bytes_requested: usize,
    },
}

/// Numeric formats supported:
/// - `0x` prefix: hexadecimal
/// - `0o` prefix: octal
/// - `0b` prefix: binary
/// - no prefix or `0d` prefix: decimal
fn split_radix_and_digits(s: &str) -> (u32, &str) {
    match s.as_bytes() {
        [b'0', b'x', ..] => (16u32, &s[2..]),
        [b'0', b'o', ..] => (8u32, &s[2..]),
        [b'0', b'b', ..] => (2u32, &s[2..]),
        [b'0', b'd', ..] => (10u32, &s[2..]),
        _ => (10u32, s),
    }
}

fn parse_prefixed_uint<T>(s: &str) -> Result<T, ParseLiteralError>
where
    T: TryFrom<u128>,
    <T as TryFrom<u128>>::Error: std::fmt::Display,
{
    let (radix, digits_raw) = split_radix_and_digits(s);

    if digits_raw.is_empty() {
        return Err(ParseLiteralError::MissingDigits {
            input: s.to_string(),
        });
    }

    // Only ignore '_' in the digits portion; underscores in the prefix area will
    // prevent matching in split_radix_and_digits and fall back to base-10.
    let digits: String = digits_raw.chars().filter(|&c| c != '_').collect();

    if digits.is_empty() {
        return Err(ParseLiteralError::MissingDigits {
            input: s.to_string(),
        });
    }

    let v = u128::from_str_radix(&digits, radix).map_err(|e| ParseLiteralError::InvalidNumber {
        input: s.to_string(),
        source: e,
    })?;

    T::try_from(v).map_err(|e| ParseLiteralError::OutOfRange {
        input: s.to_string(),
        details: e.to_string(),
    })
}

/// Parse a single integer literal into a byte vector, where the *digit count* (after removing `_`)
/// determines the output length.
///
/// Rules:
/// - Same radix/prefix rules as other integers (`0x`, `0o`, `0b`, `0d`, or no prefix for decimal).
/// - `_` is ignored in the digits.
/// - Output byte length is `ceil(digit_bits / 8)` where `digit_bits = digits_len * log2(radix)`.
///   Concretely:
///   - hex: 2 digits per byte
///   - octal: 3 digits per byte (since 3 bits per digit)
///   - binary: 8 digits per byte
///   - decimal: uses bit-length of the parsed integer (no fixed digit->bit mapping)
///
/// Endianness: big-endian (most significant byte first).
fn parse_byte_vec(s: &str) -> Result<Vec<u8>, ParseLiteralError> {
    parse_vec_u8_from_prefixed_literal(s)
}

fn parse_vec_u8_from_prefixed_literal(s: &str) -> Result<Vec<u8>, ParseLiteralError> {
    let (radix, digits_raw) = split_radix_and_digits(s);
    if digits_raw.is_empty() {
        return Err(ParseLiteralError::MissingDigits {
            input: s.to_string(),
        });
    }

    let digits: String = digits_raw.chars().filter(|&c| c != '_').collect();
    if digits.is_empty() {
        return Err(ParseLiteralError::MissingDigits {
            input: s.to_string(),
        });
    }

    // Parse value (bounded to u128 for now).
    let v = u128::from_str_radix(&digits, radix).map_err(|e| ParseLiteralError::InvalidNumber {
        input: s.to_string(),
        source: e,
    })?;

    // Determine output byte count.
    let byte_len = match radix {
        16 => (digits.len() + 1) / 2,    // 2 hex digits per byte
        8 => (digits.len() * 3 + 7) / 8, // 3 bits per oct digit
        2 => (digits.len() + 7) / 8,     // 1 bit per bin digit
        10 => {
            // Decimal has no fixed digit->bit mapping; interpret "length decides bytes"
            // as "use minimal number of bytes to represent the parsed value", but allow
            // "0" to be 1 byte.
            let bits = if v == 0 {
                1
            } else {
                128 - v.leading_zeros() as usize
            };
            (bits + 7) / 8
        }
        _ => {
            return Err(ParseLiteralError::InvalidRadix {
                input: s.to_string(),
                radix,
            });
        }
    };

    if byte_len == 0 {
        return Err(ParseLiteralError::MissingDigits {
            input: s.to_string(),
        });
    }
    if byte_len > 16 {
        return Err(ParseLiteralError::TooManyBytes {
            input: s.to_string(),
            bytes_requested: byte_len,
        });
    }

    // Big-endian, fixed length determined by digits. Leading zeros are preserved by length.
    let mut out = vec![0u8; byte_len];
    for i in 0..byte_len {
        let shift = 8 * (byte_len - 1 - i);
        out[i] = ((v >> shift) & 0xff) as u8;
    }
    Ok(out)
}

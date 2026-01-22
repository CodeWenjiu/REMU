use thiserror::Error;
use winnow::Parser as _;
use winnow::combinator::{alt, eof, opt};
use winnow::error::{ContextError, ErrMode};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Radix {
    Bin,
    Oct,
    Dec,
    Hex,
}

impl Radix {
    fn as_u32(self) -> u32 {
        match self {
            Radix::Bin => 2,
            Radix::Oct => 8,
            Radix::Dec => 10,
            Radix::Hex => 16,
        }
    }

    fn value_of(self, b: u8) -> Option<u8> {
        match self {
            Radix::Bin => match b {
                b'0'..=b'1' => Some(b - b'0'),
                _ => None,
            },
            Radix::Oct => match b {
                b'0'..=b'7' => Some(b - b'0'),
                _ => None,
            },
            Radix::Dec => match b {
                b'0'..=b'9' => Some(b - b'0'),
                _ => None,
            },
            Radix::Hex => match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(10 + (b - b'a')),
                b'A'..=b'F' => Some(10 + (b - b'A')),
                _ => None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// A string literal (only used for byte-vector parsing, by design).
    QuotedString(String),
    /// A numeric literal with a radix and its digits (underscores removed).
    Number { radix: Radix, digits: String },
}

fn parse_token(input: &mut &str) -> Result<Token, ErrMode<ContextError>> {
    // We parse either:
    // - `"...."`  (no escapes; contents can be any chars except `"`)
    // - number: [0x|0o|0b|0d]? DIGITS/_ with radix-specific digit set (underscores allowed)
    //
    // We keep this parsing local and then map errors back to ParseLiteralError.

    // 1) Quoted string: " ... "  (no escapes)
    if input.starts_with('"') {
        // consume opening quote
        *input = &input[1..];

        // find closing quote
        if let Some(end) = input.find('"') {
            let inner = input[..end].to_string();
            *input = &input[end + 1..];

            // Ensure we've consumed the entire token.
            eof.parse_next(input)?;

            return Ok(Token::QuotedString(inner));
        }

        // Unclosed quote
        return Err(ErrMode::Cut(ContextError::new()));
    }

    // 2) Number: optional prefix + digits (underscores allowed, but at least one digit required)
    let radix = opt(("0", alt(("x", "o", "b", "d"))))
        .map(|opt| match opt {
            Some((_, "x")) => Radix::Hex,
            Some((_, "o")) => Radix::Oct,
            Some((_, "b")) => Radix::Bin,
            Some((_, "d")) => Radix::Dec,
            _ => Radix::Dec,
        })
        .parse_next(input)?;

    let mut digits = String::new();
    let mut seen_digit = false;

    while !input.is_empty() {
        let b = input.as_bytes()[0];

        if b == b'_' {
            *input = &input[1..];
            continue;
        }

        if radix.value_of(b).is_some() {
            seen_digit = true;
            digits.push(b as char);
            *input = &input[1..];
            continue;
        }

        break;
    }

    if !seen_digit {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    // Ensure we've consumed the entire token.
    eof.parse_next(input)?;

    Ok(Token::Number { radix, digits })
}

/// Parse a numeric literal token into a u128.
///
/// - Supports optional prefixes: 0x/0o/0b/0d; no prefix = decimal.
/// - Supports '_' separators in the digits (already stripped by the tokenizer).
fn parse_u128_from_token(
    input_str: &str,
    radix: Radix,
    digits: &str,
) -> Result<u128, ParseLiteralError> {
    u128::from_str_radix(digits, radix.as_u32()).map_err(|e| ParseLiteralError::InvalidNumber {
        input: input_str.to_string(),
        source: e,
    })
}

/// Parse a single integer literal into a byte vector (little-endian), where the *digit count*
/// (after removing `_`) determines the output length.
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
/// Endianness: little-endian (least significant byte first).
fn parse_vec_u8_from_prefixed_literal(s: &str) -> Result<Vec<u8>, ParseLiteralError> {
    // String literal support (only for byte-vector parsing):
    // - Must be wrapped in double quotes, e.g. "deadbeef"
    // - Contents are written verbatim as UTF-8 bytes (no unescape processing)
    // - Does NOT affect integer parsing APIs (e.g. parse_prefixed_uint)
    if let Ok(tok) = parse_token.parse(s) {
        match tok {
            Token::QuotedString(inner) => return Ok(inner.as_bytes().to_vec()),
            Token::Number { radix, digits } => {
                // Parse numeric value (bounded to u128 for now).
                let v = parse_u128_from_token(s, radix, &digits)?;

                // Determine output byte count (preserve leading zeros by digit-count derived length).
                let byte_len = match radix {
                    Radix::Hex => (digits.len() + 1) / 2, // 2 hex digits per byte
                    Radix::Oct => (digits.len() * 3 + 7) / 8, // 3 bits per oct digit
                    Radix::Bin => (digits.len() + 7) / 8, // 1 bit per bin digit
                    Radix::Dec => {
                        let bits = if v == 0 {
                            1
                        } else {
                            128 - v.leading_zeros() as usize
                        };
                        (bits + 7) / 8
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

                let mut out = vec![0u8; byte_len];
                for i in 0..byte_len {
                    let shift = 8 * i;
                    out[i] = ((v >> shift) & 0xff) as u8;
                }
                return Ok(out);
            }
        }
    }

    // If token parsing failed, try to provide a compatible error surface:
    // - Empty or only underscores should be MissingDigits.
    // - Otherwise treat as InvalidNumber to match prior behavior as closely as possible.
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(ParseLiteralError::MissingDigits {
            input: s.to_string(),
        });
    }

    // As a fallback, attempt a decimal parse so we can surface ParseIntError details.
    let digits: String = trimmed.chars().filter(|&c| c != '_').collect();
    if digits.is_empty() {
        return Err(ParseLiteralError::MissingDigits {
            input: s.to_string(),
        });
    }

    let _ = u128::from_str_radix(&digits, 10).map_err(|e| ParseLiteralError::InvalidNumber {
        input: s.to_string(),
        source: e,
    })?;

    // Shouldn't reach here because successful parse would have been handled above.
    Err(ParseLiteralError::InvalidRadix {
        input: s.to_string(),
        radix: 10,
    })
}

/// Parse a prefixed integer string into an unsigned integer type.
///
/// Supported formats:
/// - `0x` prefix: hexadecimal
/// - `0o` prefix: octal
/// - `0b` prefix: binary
/// - no prefix or `0d` prefix: decimal
///
/// `_` is allowed in the digit portion and is ignored.
///
/// NOTE: This API intentionally does NOT support string literals.
pub fn parse_prefixed_uint<T>(s: &str) -> Result<T, ParseLiteralError>
where
    T: TryFrom<u128>,
    <T as TryFrom<u128>>::Error: std::fmt::Display,
{
    // parse_token enforces full consumption; we additionally reject quoted strings here
    // to preserve the existing non-string behavior.
    let tok = parse_token.parse(s).map_err(|_| {
        // Keep old-style errors as much as possible by attempting a decimal parse:
        let digits: String = s.chars().filter(|&c| c != '_').collect();
        if digits.is_empty() {
            return ParseLiteralError::MissingDigits {
                input: s.to_string(),
            };
        }
        let e = u128::from_str_radix(&digits, 10).unwrap_err();
        ParseLiteralError::InvalidNumber {
            input: s.to_string(),
            source: e,
        }
    })?;

    let (radix, digits) = match tok {
        Token::QuotedString(_) => {
            // Must not match strings for this API.
            // We mimic the prior behavior by producing an InvalidNumber error.
            let e = u128::from_str_radix("x", 10).unwrap_err();
            return Err(ParseLiteralError::InvalidNumber {
                input: s.to_string(),
                source: e,
            });
        }
        Token::Number { radix, digits } => (radix, digits),
    };

    // digits must be present; parse_token already enforces that.
    let v = parse_u128_from_token(s, radix, &digits)?;

    T::try_from(v).map_err(|e| ParseLiteralError::OutOfRange {
        input: s.to_string(),
        details: e.to_string(),
    })
}

/// Parse a single integer literal into a byte vector.
///
/// This API is used by clap value parsing for `state set` and similar.
///
/// Supported:
/// - numeric literals (see `parse_prefixed_uint` rules)
/// - quoted string literals: `"deadbeef"` → UTF-8 bytes
pub fn parse_byte_vec(s: &str) -> Result<Vec<u8>, ParseLiteralError> {
    parse_vec_u8_from_prefixed_literal(s)
}

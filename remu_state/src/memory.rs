use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemRegionSpec {
    pub name: String,
    pub base: u64,
    pub size: u64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MemRegionSpecParseError {
    #[error("empty mem region spec")]
    Empty,

    #[error("invalid mem region spec: missing '@' (expected <name>@<base>:<size>)")]
    MissingAt,

    #[error("invalid mem region spec: empty name before '@'")]
    EmptyName,

    #[error("invalid mem region spec: missing ':' (expected <name>@<base>:<size>)")]
    MissingColon,

    #[error("invalid mem region spec: empty {field}")]
    EmptyField { field: &'static str },

    #[error("invalid mem region spec: {field} '{raw}' is not valid hex: {error}")]
    InvalidHex {
        field: &'static str,
        raw: String,
        error: String,
    },

    #[error("invalid mem region spec: {field} '{raw}' is not valid decimal: {error}")]
    InvalidDec {
        field: &'static str,
        raw: String,
        error: String,
    },

    #[error("invalid mem region spec: size must be > 0")]
    SizeZero,
}

impl std::str::FromStr for MemRegionSpec {
    type Err = MemRegionSpecParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Expected format: "<name>@<base>:<size>"
        // Example: "ram1@0x8000_0000:0x0800_0000"
        let input = s.trim();
        if input.is_empty() {
            return Err(MemRegionSpecParseError::Empty);
        }

        let (name, rest) = input
            .split_once('@')
            .ok_or(MemRegionSpecParseError::MissingAt)?;

        let name = name.trim();
        if name.is_empty() {
            return Err(MemRegionSpecParseError::EmptyName);
        }

        let (base_str, size_str) = rest
            .split_once(':')
            .ok_or(MemRegionSpecParseError::MissingColon)?;

        fn parse_u64_allow_hex_underscore(
            s: &str,
            field: &'static str,
        ) -> Result<u64, MemRegionSpecParseError> {
            let raw = s.trim();
            if raw.is_empty() {
                return Err(MemRegionSpecParseError::EmptyField { field });
            }
            let cleaned: String = raw.chars().filter(|&c| c != '_').collect();

            // Accept:
            // - 0x... / 0X... hex
            // - bare hex (e.g. "8000_0000")
            // - decimal (digits only)
            let value = if let Some(hex) = cleaned
                .strip_prefix("0x")
                .or_else(|| cleaned.strip_prefix("0X"))
            {
                u64::from_str_radix(hex, 16).map_err(|e| MemRegionSpecParseError::InvalidHex {
                    field,
                    raw: raw.to_string(),
                    error: e.to_string(),
                })?
            } else if cleaned.chars().all(|c| c.is_ascii_digit()) {
                cleaned
                    .parse::<u64>()
                    .map_err(|e| MemRegionSpecParseError::InvalidDec {
                        field,
                        raw: raw.to_string(),
                        error: e.to_string(),
                    })?
            } else {
                u64::from_str_radix(&cleaned, 16).map_err(|e| {
                    MemRegionSpecParseError::InvalidHex {
                        field,
                        raw: raw.to_string(),
                        error: e.to_string(),
                    }
                })?
            };

            Ok(value)
        }

        let base = parse_u64_allow_hex_underscore(base_str, "base")?;
        let size = parse_u64_allow_hex_underscore(size_str, "size")?;

        if size == 0 {
            return Err(MemRegionSpecParseError::SizeZero);
        }

        Ok(MemRegionSpec {
            name: name.to_string(),
            base,
            size,
        })
    }
}

#[derive(clap::Args, Debug)]
pub struct StateOption {
    #[arg(
        long = "mem",
        value_name = "NAME@BASE:SIZE",
        action = clap::ArgAction::Append,
        default_value = "ram@0x8000_0000:0x0800_0000"
    )]
    pub mem: Vec<MemRegionSpec>,
}

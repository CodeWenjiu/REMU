//! Parse `riscv32*` app target shorthands: base arch + optional **named** extensions
//! (`_zve32x_zvl128b`, …). Single-letter RISC-V names in a triple are handled by
//! Rust's built-in targets (`riscv32im-unknown-none-elf`); here we only handle **multi-token**
//! extension suffixes joined with `_`.

use winnow::combinator::{alt, preceded};
use winnow::prelude::*;
use winnow::token::literal;

/// A named extension segment after `_`, not representable as one MISA letter (cf. `ExtensionSpec` in remu_isa).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum NamedExtension {
    Zve32xZvl128b,
}

impl NamedExtension {
    /// Canonical shorthand segment, e.g. `zve32x_zvl128b` (used in declared
    /// target lists and error messages).
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Zve32xZvl128b => "zve32x_zvl128b",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedAppShorthand {
    /// e.g. `riscv32im` (no `-unknown-none-elf` suffix).
    pub base_prefix: String,
    pub extensions: Vec<NamedExtension>,
}

fn rv_base(input: &mut &str) -> ModalResult<&'static str> {
    alt((
        // Longer prefixes first (winnow matches literals in order).
        literal("riscv64imac").value("riscv64imac"),
        literal("riscv32imac").value("riscv32imac"),
        literal("riscv64im").value("riscv64im"),
        literal("riscv32im").value("riscv32im"),
        literal("riscv64i").value("riscv64i"),
        literal("riscv32i").value("riscv32i"),
    ))
    .parse_next(input)
}

fn underscore_zve32x_zvl128b(input: &mut &str) -> ModalResult<NamedExtension> {
    preceded('_', literal("zve32x_zvl128b"))
        .value(NamedExtension::Zve32xZvl128b)
        .parse_next(input)
}

fn named_extension_segment(input: &mut &str) -> ModalResult<NamedExtension> {
    // Longest literal first when adding more `alt` arms.
    alt((underscore_zve32x_zvl128b,)).parse_next(input)
}

/// If `key` does not start with a known `riscv*` base, returns `Ok(None)` so callers can fall back
/// to legacy `expand_builtin` (e.g. hypothetical custom short names).
///
/// If it starts with `riscv*` but trailing segments are invalid, returns `Err`.
pub(crate) fn parse_riscv_app_shorthand(key: &str) -> Result<Option<ParsedAppShorthand>, String> {
    let mut input = key;
    let base = match rv_base.parse_next(&mut input) {
        Ok(b) => b.to_string(),
        Err(_) => return Ok(None),
    };

    let mut extensions = Vec::new();
    while !input.is_empty() {
        let ext = named_extension_segment
            .parse_next(&mut input)
            .map_err(|_| {
                format!(
                    "invalid app target shorthand {key:?}: expected a known named extension after {:?} (e.g. _zve32x_zvl128b)",
                    base
                )
            })?;
        extensions.push(ext);
    }

    if !input.is_empty() {
        return Err(format!(
            "xtask internal: leftover input {input:?} after parsing app target shorthand {key:?}"
        ));
    }

    Ok(Some(ParsedAppShorthand {
        base_prefix: base,
        extensions,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riscv32im_plain() {
        let p = parse_riscv_app_shorthand("riscv32im").unwrap().unwrap();
        assert_eq!(p.base_prefix, "riscv32im");
        assert!(p.extensions.is_empty());
    }

    #[test]
    fn zve_shorthand() {
        let p = parse_riscv_app_shorthand("riscv32im_zve32x_zvl128b")
            .unwrap()
            .unwrap();
        assert_eq!(p.base_prefix, "riscv32im");
        assert_eq!(p.extensions, vec![NamedExtension::Zve32xZvl128b]);
    }

    #[test]
    fn unknown_riscv_suffix_errors() {
        assert!(parse_riscv_app_shorthand("riscv32im_foo").is_err());
    }

    #[test]
    fn non_riscv_fallback_none() {
        assert!(parse_riscv_app_shorthand("foo").unwrap().is_none());
    }
}

#[cfg(test)]
mod rv64_tests {
    use super::*;

    #[test]
    fn riscv64im_plain() {
        let p = parse_riscv_app_shorthand("riscv64im").unwrap().unwrap();
        assert_eq!(p.base_prefix, "riscv64im");
        assert!(p.extensions.is_empty());
    }

    #[test]
    fn riscv64i_plain() {
        let p = parse_riscv_app_shorthand("riscv64i").unwrap().unwrap();
        assert_eq!(p.base_prefix, "riscv64i");
        assert!(p.extensions.is_empty());
    }

    #[test]
    fn longer_prefix_wins() {
        // "riscv64im" must not be parsed as "riscv64i" + leftover "m".
        let p = parse_riscv_app_shorthand("riscv64imac").unwrap().unwrap();
        assert_eq!(p.base_prefix, "riscv64imac");
        let p = parse_riscv_app_shorthand("riscv64im").unwrap().unwrap();
        assert_eq!(p.base_prefix, "riscv64im");
    }

    #[test]
    fn rv64_with_unknown_suffix_errors() {
        assert!(parse_riscv_app_shorthand("riscv64im_foo").is_err());
    }
}

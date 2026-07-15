//! Parse `riscv32*` app target shorthands: base arch + optional **named** extensions
//! (`_zve32x_zvl128b`, `_wjCus0`, …). Single-letter RISC-V names in a triple are handled by
//! Rust's built-in targets (`riscv32im-unknown-none-elf`); here we only handle **multi-token**
//! extension suffixes joined with `_`.

use winnow::combinator::{alt, preceded};
use winnow::prelude::*;
use winnow::token::literal;

/// A named extension segment after `_`, not representable as one MISA letter (cf. `ExtensionSpec` in remu_types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum NamedExtension {
    Zve32xZvl128b,
    WjCus0,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedAppShorthand {
    /// e.g. `riscv32im` (no `-unknown-none-elf` suffix).
    pub base_prefix: String,
    pub extensions: Vec<NamedExtension>,
}

fn rv32_base(input: &mut &str) -> ModalResult<&'static str> {
    alt((
        literal("riscv32imac").value("riscv32imac"),
        literal("riscv32im").value("riscv32im"),
        literal("riscv32i").value("riscv32i"),
    ))
    .parse_next(input)
}

fn underscore_zve32x_zvl128b(input: &mut &str) -> ModalResult<NamedExtension> {
    preceded('_', literal("zve32x_zvl128b"))
        .value(NamedExtension::Zve32xZvl128b)
        .parse_next(input)
}

/// Matches `_wjCus0` (ASCII case variations; aligned with `ExtensionSpec::WjCus0` parsing).
fn underscore_wj_cus0(input: &mut &str) -> ModalResult<NamedExtension> {
    preceded(
        '_',
        alt((literal("wjCus0"), literal("wjcus0"), literal("WJCUS0"))),
    )
    .void()
    .value(NamedExtension::WjCus0)
    .parse_next(input)
}

fn named_extension_segment(input: &mut &str) -> ModalResult<NamedExtension> {
    // Longest literal first when adding more `alt` arms.
    alt((underscore_zve32x_zvl128b, underscore_wj_cus0)).parse_next(input)
}

/// If `key` does not start with a known `riscv32*` base, returns `Ok(None)` so callers can fall back
/// to legacy `expand_builtin` (e.g. hypothetical custom short names).
///
/// If it starts with `riscv32*` but trailing segments are invalid, returns `Err`.
pub(crate) fn parse_riscv_app_shorthand(key: &str) -> Result<Option<ParsedAppShorthand>, String> {
    let mut input = key;
    let base = match rv32_base.parse_next(&mut input) {
        Ok(b) => b.to_string(),
        Err(_) => return Ok(None),
    };

    let mut extensions = Vec::new();
    while !input.is_empty() {
        let ext = named_extension_segment
            .parse_next(&mut input)
            .map_err(|_| {
                format!(
                    "invalid app target shorthand {key:?}: expected a known named extension after {:?} (e.g. _zve32x_zvl128b, _wjCus0)",
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
    fn riscv32im_wj_cus0() {
        let p = parse_riscv_app_shorthand("riscv32im_wjCus0")
            .unwrap()
            .unwrap();
        assert_eq!(p.base_prefix, "riscv32im");
        assert_eq!(p.extensions, vec![NamedExtension::WjCus0]);
    }

    #[test]
    fn riscv32i_wj_cus0_case_insensitive() {
        let p = parse_riscv_app_shorthand("riscv32i_WJCUS0")
            .unwrap()
            .unwrap();
        assert_eq!(p.base_prefix, "riscv32i");
        assert_eq!(p.extensions, vec![NamedExtension::WjCus0]);
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

    #[test]
    fn zve_then_wj_parses_two_segments() {
        let p = parse_riscv_app_shorthand("riscv32im_zve32x_zvl128b_wjCus0")
            .unwrap()
            .unwrap();
        assert_eq!(p.extensions.len(), 2);
    }
}

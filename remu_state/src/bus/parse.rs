pub(crate) fn parse_usize_allow_hex_underscore(s: &str, field: &str) -> Result<usize, String> {
    let raw = s.trim();
    if raw.is_empty() {
        return Err(format!("invalid mem region spec: empty {}", field));
    }

    // allow '_' inside numbers
    let cleaned: String = raw.chars().filter(|&c| c != '_').collect();

    // Accept:
    // - 0x... / 0X... hex
    // - bare hex (e.g. "8000_0000")
    // - decimal (digits only)
    let value = if let Some(hex) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        usize::from_str_radix(hex, 16).map_err(|e| {
            format!(
                "invalid mem region spec: {} '{}' is not valid hex: {}",
                field, raw, e
            )
        })?
    } else if cleaned.chars().all(|c| c.is_ascii_digit()) {
        cleaned.parse::<usize>().map_err(|e| {
            format!(
                "invalid mem region spec: {} '{}' is not valid decimal: {}",
                field, raw, e
            )
        })?
    } else {
        usize::from_str_radix(&cleaned, 16).map_err(|e| {
            format!(
                "invalid mem region spec: {} '{}' is not valid hex: {}",
                field, raw, e
            )
        })?
    };

    Ok(value)
}

use reedline::{ValidationResult, Validator};

/// A validator for remu's command language.
///
/// Differences vs `reedline::DefaultValidator`:
/// - Only `{}` braces are considered for multiline continuation (no `()`, `[]`, quotes, etc.)
/// - Exposes a `prefix_len` so the continuation prompt can be aligned with your custom prompt.
///
/// Note:
/// The multiline continuation *text* itself is controlled by the `Prompt`
/// (`Prompt::render_prompt_multiline_indicator`). The validator only decides
/// whether the current buffer is complete.
/// If you also want the printed prefix to be exactly `prefix_len` characters,
/// implement a custom prompt and return `" ".repeat(prefix_len)` or similar
/// from `render_prompt_multiline_indicator()`.
#[derive(Debug, Clone)]
pub struct RemuValidator {
    /// Intended continuation prefix width (in characters). Stored for configuration symmetry.
    /// See note above.
    pub prefix_len: usize,
}

impl RemuValidator {
    pub fn new(prefix_len: usize) -> Self {
        Self { prefix_len }
    }

    fn incomplete_braces_only(line: &str) -> bool {
        let mut depth: isize = 0;

        for c in line.chars() {
            match c {
                '{' => depth += 1,
                '}' => {
                    // If we're already at 0, this is an unmatched '}'.
                    // Treat it as "complete" from a multiline standpoint (not incomplete).
                    // The command parser will surface a proper error later.
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                _ => {}
            }
        }

        depth > 0
    }
}

impl Validator for RemuValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        if Self::incomplete_braces_only(line) {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}

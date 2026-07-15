#[must_use]
pub fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".into();
    }
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "._/-:+@".contains(c))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

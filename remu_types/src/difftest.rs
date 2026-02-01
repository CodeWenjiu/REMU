use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifftestRef {
    Remu,
}

impl FromStr for DifftestRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().eq_ignore_ascii_case("remu") {
            true => Ok(DifftestRef::Remu),
            false => Err(format!("未知的 difftest ref: '{}'，当前仅支持: remu", s)),
        }
    }
}

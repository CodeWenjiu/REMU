use std::fmt;
use std::str::FromStr;

use crate::AllUsize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifftestRef {
    Remu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegGroup {
    Pc,
    Gpr,
    Fpr,
}

#[derive(Debug, Clone)]
pub struct DifftestMismatchItem {
    pub group: RegGroup,
    pub name: String,
    pub ref_val: AllUsize,
    pub dut_val: AllUsize,
}

impl fmt::Display for DifftestMismatchItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  {} {}: ref={} dut={}",
            match self.group {
                RegGroup::Pc => "pc",
                RegGroup::Gpr => "gpr",
                RegGroup::Fpr => "fpr",
            },
            self.name,
            self.ref_val,
            self.dut_val
        )
    }
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

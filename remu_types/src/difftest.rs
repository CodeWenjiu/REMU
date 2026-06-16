use std::fmt;
use std::str::FromStr;

use crate::AllUsize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifftestRef {
    Remu,
    Spike,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifftestRegGroup {
    Pc,
    Gpr,
    Fpr,
    Vr,
    Csr,
}

impl fmt::Display for DifftestRegGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pc => "pc",
            Self::Gpr => "gpr",
            Self::Fpr => "fpr",
            Self::Vr => "vr",
            Self::Csr => "csr",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifftestGroup {
    Reg(DifftestRegGroup),
    Mem,
}

impl fmt::Display for DifftestGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reg(r) => write!(f, "reg:{}", r),
            Self::Mem => f.write_str("mem"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DifftestMismatchItem {
    pub group: DifftestGroup,
    pub name: String,
    pub ref_val: AllUsize,
    pub dut_val: AllUsize,
}

impl fmt::Display for DifftestMismatchItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  {} {}: ref={} dut={}",
            self.group, self.name, self.ref_val, self.dut_val
        )
    }
}

impl FromStr for DifftestRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_lower = s.trim().to_ascii_lowercase();
        match s_lower.as_str() {
            "remu" => Ok(DifftestRef::Remu),
            "spike" => Ok(DifftestRef::Spike),
            _ => Err(format!(
                "unknown difftest ref: '{}', supported: remu, spike",
                s
            )),
        }
    }
}

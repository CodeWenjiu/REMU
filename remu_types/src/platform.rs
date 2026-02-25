use std::str::FromStr;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    None = 0,
    Remu = 1,
    Spike = 2,
    Nzea = 3,
}

impl Platform {
    /// Returns the platform name as a static string.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Platform::None => "none",
            Platform::Remu => "remu",
            Platform::Spike => "spike",
            Platform::Nzea => "nzea",
        }
    }
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_lower = s.trim().to_lowercase();
        match s_lower.as_str() {
            "none" => Ok(Platform::None),
            "remu" => Ok(Platform::Remu),
            "spike" => Ok(Platform::Spike),
            "nzea" => Ok(Platform::Nzea),
            _ => Err(format!(
                "unknown platform: '{}', supported: none, remu, spike, nzea",
                s
            )),
        }
    }
}

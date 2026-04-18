use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NzeaTarget {
    #[default]
    Core,
    Tile,
}

impl NzeaTarget {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Tile => "tile",
        }
    }
}

impl FromStr for NzeaTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "core" => Ok(Self::Core),
            "tile" => Ok(Self::Tile),
            other => Err(format!(
                "invalid nzea target {other:?} (expected core or tile)"
            )),
        }
    }
}

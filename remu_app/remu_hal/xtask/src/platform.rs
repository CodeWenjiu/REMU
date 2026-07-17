use std::str::FromStr;

use strum::IntoEnumIterator;

/// Simulation / execution platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
pub enum Platform {
    Remu,
    Qemu,
    Spike,
    Host,
}

impl FromStr for Platform {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "remu" => Ok(Platform::Remu),
            "qemu" => Ok(Platform::Qemu),
            "spike" => Ok(Platform::Spike),
            "host" => Ok(Platform::Host),
            _ => Err(format!("unknown platform: {s}")),
        }
    }
}

/// Per-platform configuration: extra linker scripts, rustflags, etc.
pub trait PlatformConfig {
    fn rustflags(&self) -> Vec<String> {
        vec![]
    }
}

impl PlatformConfig for Platform {
    fn rustflags(&self) -> Vec<String> {
        // RISC-V baseline for all embedded platforms.
        let mut flags: Vec<String> = vec![
            "-C".into(),
            "link-arg=-Tmemory.x".into(),
            "-C".into(),
            "link-arg=-Tlink/tohost.x".into(),
            "--cfg".into(),
            format!("platform_{}", self.as_str()).into(),
        ];
        match self {
            Platform::Spike => {
                flags.extend_from_slice(&["-C".into(), "link-arg=-Tlink/htif.x".into()])
            }
            _ => {}
        }
        flags
    }
}

impl Platform {
    /// All platform names derived from the enum, for `rustc-check-cfg` declarations.
    pub fn names() -> Vec<&'static str> {
        Self::iter().map(|p| p.as_str()).collect()
    }

    fn as_str(self) -> &'static str {
        match self {
            Platform::Remu => "remu",
            Platform::Qemu => "qemu",
            Platform::Spike => "spike",
            Platform::Host => "host",
        }
    }
}

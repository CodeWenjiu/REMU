use std::str::FromStr;

use strum::IntoEnumIterator;

/// Simulation / execution platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter, Default)]
pub enum Platform {
    #[default]
    Remu,
    Qemu,
    Spike,
    Host,
    /// rcore: the app runs as a U-mode program under `rcore_kernel` (S mode,
    /// with `remu_firmware` in M mode). Applies to `run-app` / `build-app`;
    /// the firmware + kernel are injected by the recipes.
    Rcore,
}

impl FromStr for Platform {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "remu" => Ok(Platform::Remu),
            "qemu" => Ok(Platform::Qemu),
            "spike" => Ok(Platform::Spike),
            "host" => Ok(Platform::Host),
            "rcore" => Ok(Platform::Rcore),
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
        // rcore user programs are U-mode: no machine-mode linker scripts
        // (-Tmemory.x / tohost.x are M-mode bare-metal conventions); they link
        // with their own `linker.ld` (app base 0x8040_0000). Mark the platform
        // cfg so apps can select the syscall-based U-mode backend.
        if *self == Platform::Rcore {
            return vec!["--cfg".into(), "platform_rcore".into()];
        }
        // RISC-V baseline for all embedded platforms.
        let mut flags: Vec<String> = vec![
            "-C".into(),
            "link-arg=-Tmemory.x".into(),
            "-C".into(),
            "link-arg=-Tlink/tohost.x".into(),
            "--cfg".into(),
            format!("platform_{}", self.as_str()),
        ];
        if *self == Platform::Spike {
            flags.extend_from_slice(&["-C".into(), "link-arg=-Tlink/htif.x".into()])
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
            Platform::Rcore => "rcore",
        }
    }
}

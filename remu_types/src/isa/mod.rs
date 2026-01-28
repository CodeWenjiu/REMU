remu_macro::mod_pub!(reg, extension, extension_enum);

use std::str::FromStr;

use target_lexicon::{Architecture, Triple};

use crate::{Xlen, isa::extension::Extension};

pub trait ArchConfig: 'static + Copy {
    type M: Extension<State = ()>;

    type F: Extension;
}

pub trait RvIsa: 'static + Copy {
    type XLEN: Xlen;
    type Conf: ArchConfig;

    const HAS_M: bool = <Self::Conf as ArchConfig>::M::ENABLED;
    const HAS_F: bool = <Self::Conf as ArchConfig>::F::ENABLED;
}

#[derive(Debug, Clone)]
pub struct IsaSpec(pub Architecture);

impl FromStr for IsaSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized_s = if s.contains('-') {
            s.to_string()
        } else {
            format!("{}-unkonwn-none-elf", s)
        };

        let architecture = normalized_s
            .parse::<Triple>()
            .map_err(|e| format!("Invalid ISA string: '{}',: {}", s, e))?
            .architecture;

        match architecture {
            Architecture::Riscv32(_) | Architecture::Riscv64(_) => Ok(IsaSpec(architecture)),
            _ => Err(format!("Unsupported ISA architecture: {}", architecture)),
        }
    }
}

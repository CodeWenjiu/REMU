use std::str::FromStr;

use target_lexicon::{Architecture, Triple};

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

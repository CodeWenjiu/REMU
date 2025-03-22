use std::{error::Error, str::FromStr};

#[derive(Debug, Clone, Copy)]
pub enum ISA {
    RV32E,
    RV32I,
    RV32IM,
}

#[derive(Debug, Clone, Copy)]
pub enum Simulators {
    EMU,
    NPC,
}

impl From<ISA> for &str {
    fn from(isa: ISA) -> Self {
        match isa {
            ISA::RV32E => "rv32e",
            ISA::RV32I => "rv32i",
            ISA::RV32IM => "rv32im",
        }
    }
}

impl FromStr for ISA {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rv32e" => Ok(ISA::RV32E),
            "rv32i" => Ok(ISA::RV32I),
            "rv32im" => Ok(ISA::RV32IM),
            _ => Err("Unknown ISA".into()),
        }
    }
}

impl From<Simulators> for &str {
    fn from(sim: Simulators) -> Self {
        match sim {
            Simulators::EMU => "emu",
            Simulators::NPC => "npc",
        }
    }
}

impl FromStr for Simulators {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "emu" => Ok(Simulators::EMU),
            "npc" => Ok(Simulators::NPC),
            _ => Err("Unknown Simulator".into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Platform {
    pub isa: ISA,
    pub simulator: Simulators,
}

impl FromStr for Platform {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (isa, simulator) = s.split_once('-').unwrap();
        Ok(Platform {
            isa: ISA::from_str(isa)?,
            simulator: Simulators::from_str(simulator)?,
        })
    }
}

impl ToString for Platform {
    fn to_string(&self) -> String {
        format!("{}-{}", Into::<&str>::into(self.isa), Into::<&str>::into(self.simulator))
    }
}
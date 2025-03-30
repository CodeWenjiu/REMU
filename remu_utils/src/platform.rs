use std::{error::Error, str::FromStr};

#[derive(Debug, Clone, Copy)]
pub enum ISA {
    RV32E,
    RV32I,
    RV32IM,
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

#[derive(Debug, Clone, Copy)]
pub enum Simulators {
    EMU,
    NZEA,
}

impl From<Simulators> for &str {
    fn from(sim: Simulators) -> Self {
        match sim {
            Simulators::EMU => "emu",
            Simulators::NZEA => "npc",
        }
    }
}

impl FromStr for Simulators {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "emu" => Ok(Simulators::EMU),
            "nzea" => Ok(Simulators::NZEA),
            _ => Err("Unknown Simulator".into()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DifftestBuildIn {
    EMU,
}

impl std::fmt::Display for DifftestBuildIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DifftestBuildIn::EMU => write!(f, "emu"),
        }
    }
}

impl From<DifftestBuildIn> for &str {
    fn from(sim: DifftestBuildIn) -> Self {
        match sim {
            DifftestBuildIn::EMU => "emu",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DifftestFFI {
    SPIKE,
}

impl std::fmt::Display for DifftestFFI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DifftestFFI::SPIKE => write!(f, "spike"),
        }
    }
}

impl From<DifftestFFI> for &str {
    fn from(sim: DifftestFFI) -> Self {
        match sim {
            DifftestFFI::SPIKE => "spike",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DifftestRef {
    BuildIn(DifftestBuildIn),
    FFI(DifftestFFI),
}

impl std::fmt::Display for DifftestRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DifftestRef::BuildIn(sim) => write!(f, "{}", sim),
            DifftestRef::FFI(sim) => write!(f, "{}", sim),
        }
    }
}

impl From<DifftestRef> for &str {
    fn from(sim: DifftestRef) -> Self {
        match sim {
            DifftestRef::BuildIn(sim) => Into::<&str>::into(sim),
            DifftestRef::FFI(sim) => Into::<&str>::into(sim),
        }
    }
}

impl FromStr for DifftestRef {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "emu" => Ok(DifftestRef::BuildIn(DifftestBuildIn::EMU)),
            "spike" => Ok(DifftestRef::FFI(DifftestFFI::SPIKE)),
            _ => Err("Unknown DifftestRef".into()),
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
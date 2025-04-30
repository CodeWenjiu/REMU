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
            "riscv32e" => Ok(ISA::RV32E),

            "riscv32i" => Ok(ISA::RV32I),
            "rv32i" => Ok(ISA::RV32I),

            "riscv32im" => Ok(ISA::RV32IM),
            "riscv32" => Ok(ISA::RV32IM),
            "rv32im" => Ok(ISA::RV32IM),

            _ => Err("Unknown ISA".into()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RemuSimulators {
    NEMU,
}

impl From<RemuSimulators> for &str {
    fn from(sim: RemuSimulators) -> Self {
        match sim {
            RemuSimulators::NEMU => "nemu",
        }
    }
}

impl FromStr for RemuSimulators {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "nemu" => Ok(RemuSimulators::NEMU),
            _ => Err("Unknown Remu Simulator".into()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NzeaSimulators {
    NPC,
    YSYXSOC,
    JYD,
    JydRemote,
}

impl From<NzeaSimulators> for &str {
    fn from(sim: NzeaSimulators) -> Self {
        match sim {
            NzeaSimulators::NPC => "npc",
            NzeaSimulators::YSYXSOC => "ysyxsoc",
            NzeaSimulators::JYD => "jyd",
            NzeaSimulators::JydRemote => "jyd_remote",
        }
    }
}

impl FromStr for NzeaSimulators {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "npc" => Ok(NzeaSimulators::NPC),
            "ysyxsoc" => Ok(NzeaSimulators::YSYXSOC),
            "jyd" => Ok(NzeaSimulators::JYD),
            "jyd_remote" => Ok(NzeaSimulators::JydRemote),
            _ => Err("Unknown Nzea Simulator".into()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Simulators {
    EMU (RemuSimulators),
    NZEA (NzeaSimulators),
}

impl From<Simulators> for &str {
    fn from(sim: Simulators) -> Self {
        match sim {
            Simulators::EMU(remu_sim) => {
                Box::leak(format!("emu-{}", Into::<&str>::into(remu_sim)).into_boxed_str())
            }
            Simulators::NZEA(nzea_sim) => {
                Box::leak(format!("nzea-{}", Into::<&str>::into(nzea_sim)).into_boxed_str())
            }
        }
    }
}

impl FromStr for Simulators {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (sim_type, sim_name) = s.split_once('-').ok_or("Invalid simulator format")?;

        match sim_type {
            "emu" => Ok(Simulators::EMU(RemuSimulators::from_str(sim_name)?)),
            "nzea" => Ok(Simulators::NZEA(NzeaSimulators::from_str(sim_name)?)),
            _ => Err("Unknown simulator type".into()),
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
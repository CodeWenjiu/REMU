use std::{error::Error, str::FromStr, path::Path};

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
pub enum EmuSimulators {
    DM,
    SC,
    PL,
}

impl From<EmuSimulators> for &str {
    fn from(sim: EmuSimulators) -> Self {
        match sim {
            EmuSimulators::DM => "Dm",
            EmuSimulators::SC => "Sc",
            EmuSimulators::PL => "Pl",
        }
    }
}

impl FromStr for EmuSimulators {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dm" => Ok(EmuSimulators::DM),
            "sc" => Ok(EmuSimulators::SC),
            "pl" => Ok(EmuSimulators::PL),
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
            NzeaSimulators::NPC => "Npc",
            NzeaSimulators::YSYXSOC => "Ysyxsoc",
            NzeaSimulators::JYD => "Jyd",
            NzeaSimulators::JydRemote => "JydRemote",
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
    EMU (EmuSimulators),
    NZEA (NzeaSimulators),
}

impl From<Simulators> for &str {
    fn from(sim: Simulators) -> Self {
        match sim {
            Simulators::EMU(remu_sim) => {
                Box::leak(format!("Emu-{}", Into::<&str>::into(remu_sim)).into_boxed_str())
            }
            Simulators::NZEA(nzea_sim) => {
                Box::leak(format!("Nzea-{}", Into::<&str>::into(nzea_sim)).into_boxed_str())
            }
        }
    }
}

impl FromStr for Simulators {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (sim_type, sim_name) = s.split_once('-').ok_or("Invalid simulator format")?;

        match sim_type {
            "emu" => Ok(Simulators::EMU(EmuSimulators::from_str(sim_name)?)),
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
    TARGET,
}

impl std::fmt::Display for DifftestFFI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DifftestFFI::TARGET => write!(f, "ffi"),
        }
    }
}

impl From<DifftestFFI> for &str {
    fn from(sim: DifftestFFI) -> Self {
        match sim {
            DifftestFFI::TARGET => "ffi",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DifftestRef {
    BuildIn(DifftestBuildIn),
    FFI(&'static str),
}

impl DifftestRef {
    pub fn new_ffi(path: &'static str) -> Result<Self, String> {
        if Path::new(path).exists() {
            Ok(DifftestRef::FFI(path))
        } else {
            Err(format!("FFI path: {} not found while not an valid buildin ref", path))
        }
    }
}

impl std::fmt::Display for DifftestRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DifftestRef::BuildIn(sim) => write!(f, "{}", sim),
            DifftestRef::FFI(sim) => write!(f, "{}", sim),
        }
    }
}

impl From<DifftestRef> for String {
    fn from(sim: DifftestRef) -> Self {
        match sim {
            DifftestRef::BuildIn(sim) => Into::<&str>::into(sim).to_string(),
            DifftestRef::FFI(sim) => sim.to_owned(),
        }
    }
}

impl FromStr for DifftestRef {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "emu" => Ok(DifftestRef::BuildIn(DifftestBuildIn::EMU)),
            path => DifftestRef::new_ffi(path.to_string().leak())
                .map_err(|e| e.into())
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
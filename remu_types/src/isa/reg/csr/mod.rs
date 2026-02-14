//! CSR kinds (enum + Mcause) and vector CSR state interface.

mod vector;
pub use vector::*;

use strum::{Display, EnumString, FromRepr};

#[derive(Debug, PartialEq, Clone, Copy, Eq, FromRepr)]
#[repr(u32)]
pub enum Mcause {
    // Synchronous exceptions (bit 31 = 0)
    InstructionAddressMisaligned = 0,
    InstructionAccessFault = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadAddressMisaligned = 4,
    LoadAccessFault = 5,
    StoreAddressMisaligned = 6,
    StoreAccessFault = 7,
    EnvCallFromU = 8,
    EnvCallFromS = 9,
    EnvCallFromM = 11,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
    // Interrupts (bit 31 = 1)
    MachineSoftwareInterrupt = 0x8000_0003,
    MachineTimerInterrupt = 0x8000_0007,
    MachineExternalInterrupt = 0x8000_000B,
}

impl Mcause {
    #[inline(always)]
    pub fn to_u32(self) -> u32 {
        self as u32
    }

    #[inline(always)]
    pub fn from_u32(x: u32) -> Option<Self> {
        Self::from_repr(x)
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Eq, EnumString, Display, FromRepr)]
#[repr(u16)]
#[strum(ascii_case_insensitive)]
pub enum Csr {
    // Machine Information
    #[strum(to_string = "mvendorid", serialize = "mvendorid")]
    Mvendorid = 0xF11,
    #[strum(to_string = "marchid", serialize = "marchid")]
    Marchid = 0xF12,
    #[strum(to_string = "mimpid", serialize = "mimpid")]
    Mimpid = 0xF13,
    #[strum(to_string = "mhartid", serialize = "mhartid")]
    Mhartid = 0xF14,

    // Machine Trap Setup
    #[strum(to_string = "mstatus", serialize = "mstatus")]
    Mstatus = 0x300,
    #[strum(to_string = "misa", serialize = "misa")]
    Misa = 0x301,
    #[strum(to_string = "medeleg", serialize = "medeleg")]
    Medeleg = 0x302,
    #[strum(to_string = "mideleg", serialize = "mideleg")]
    Mideleg = 0x303,
    #[strum(to_string = "mie", serialize = "mie")]
    Mie = 0x304,
    #[strum(to_string = "mtvec", serialize = "mtvec")]
    Mtvec = 0x305,
    #[strum(to_string = "mcounteren", serialize = "mcounteren")]
    Mcounteren = 0x306,

    // Machine Trap Handling
    #[strum(to_string = "mscratch", serialize = "mscratch")]
    Mscratch = 0x340,
    #[strum(to_string = "mepc", serialize = "mepc")]
    Mepc = 0x341,
    #[strum(to_string = "mcause", serialize = "mcause")]
    Mcause = 0x342,
    #[strum(to_string = "mtval", serialize = "mtval")]
    Mtval = 0x343,
    #[strum(to_string = "mip", serialize = "mip")]
    Mip = 0x344,

    // Machine Counter/Timer (RV32: low 0xB00/0xB02, high 0xB80/0xB82)
    #[strum(to_string = "mcycle", serialize = "mcycle")]
    Mcycle = 0xB00,
    #[strum(to_string = "minstret", serialize = "minstret")]
    Minstret = 0xB02,
    #[strum(to_string = "mcycleh", serialize = "mcycleh")]
    Mcycleh = 0xB80,
    #[strum(to_string = "minstreth", serialize = "minstreth")]
    Minstreth = 0xB82,

    // Vector (Zve32x) CSRs
    #[strum(to_string = "vstart", serialize = "vstart")]
    Vstart = 0x008,
    #[strum(to_string = "vxsat", serialize = "vxsat")]
    Vxsat = 0x009,
    #[strum(to_string = "vxrm", serialize = "vxrm")]
    Vxrm = 0x00A,
    #[strum(to_string = "vcsr", serialize = "vcsr")]
    Vcsr = 0x00F,
    #[strum(to_string = "vl", serialize = "vl")]
    Vl = 0xC20,
    #[strum(to_string = "vtype", serialize = "vtype")]
    Vtype = 0xC21,
    #[strum(to_string = "vlenb", serialize = "vlenb")]
    Vlenb = 0xC22,
}

impl Csr {
    #[inline(always)]
    pub fn addr(self) -> u16 {
        self as u16
    }

    #[inline(always)]
    pub fn idx(self) -> usize {
        self.addr() as usize
    }

    /// CSRs that have concrete state (stored in reg file), as opposed to read-only CSRs
    /// like Misa that are determined by ISA.
    #[inline(always)]
    pub fn csrs_with_state() -> &'static [Csr] {
        use Csr::*;
        const CSRS: &[Csr] = &[
            Mstatus, Mie, Mtvec, Mscratch, Mepc, Mcause, Mtval, Mip, Vstart, Vxsat, Vxrm, Vcsr, Vl,
            Vtype,
        ];
        CSRS
    }

    /// Mask for difftest: bits to compare. 0 = skip this CSR (platform/impl-defined or counter).
    /// Compare passes when (ref_val & mask) == (dut_val & mask).
    #[inline(always)]
    pub fn diff_mask(self) -> u32 {
        use Csr::*;
        match self {
            Mvendorid | Marchid | Mimpid | Mhartid => 0,
            Mstatus => {
                // RV32: mask off SD (bit 31) and WPRI/reserved. Compare MIE, MPIE, MPP, SPP, SIE, etc.
                0x0000_1888
            }
            Misa | Mie | Mtvec | Mscratch | Mepc | Mcause | Mtval | Mip => 0xFFFF_FFFF,
            Medeleg | Mideleg | Mcounteren => 0,
            Mcycle | Minstret | Mcycleh | Minstreth => 0,
            Vstart | Vl | Vtype => 0xFFFF_FFFF,
            Vxsat => 0x1,
            Vxrm => 0x3,
            Vcsr => 0x7,
            Vlenb => 0xFFFF_FFFF,
        }
    }
}

// Difftest CSR lists: incremental. All ISAs get the base; extensions (e.g. V) add their slices.

use Csr::*;

/// Base CSRs for difftest (all ISAs): Misa + machine trap/state. Always included.
pub const CSRS_FOR_DIFFTEST_BASE: &[Csr] = &[
    Misa, Mstatus, Mie, Mtvec, Mscratch, Mepc, Mcause, Mtval, Mip,
];

/// Vector CSRs for difftest. Only included when V present; add this slice on top of base.
pub const CSRS_FOR_DIFFTEST_V: &[Csr] = &[Vstart, Vxsat, Vxrm, Vcsr, Vl, Vtype, Vlenb];

/// One segment: base only. Default for ISAs without V.
pub static DIFFTEST_SLICES_BASE: &[&[Csr]] = &[CSRS_FOR_DIFFTEST_BASE];

/// Two segments: base + V. Use when VConfig::VLENB != 0.
pub static DIFFTEST_SLICES_BASE_AND_V: &[&[Csr]] = &[CSRS_FOR_DIFFTEST_BASE, CSRS_FOR_DIFFTEST_V];

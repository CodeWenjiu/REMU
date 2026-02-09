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
}

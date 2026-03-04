//! Trace flag bits: each bit of u64 controls a trace option, avoiding generic constant explosion.

/// Trace option kinds; each variant corresponds to a bit position in the TRACE u64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TraceKind {
    /// Bit 0: instruction trace (disassembly)
    Instruction = 0,
    /// Bit 1: waveform trace
    Wavetrace = 1,
}

impl TraceKind {
    #[inline(always)]
    pub const fn bit(self) -> u64 {
        1 << (self as u32)
    }
}

/// Trace flag bits. Each bit corresponds to one trace feature.
///
/// Bit layout:
/// - 0: Instruction trace (disassembly)
/// - 1: Wave trace (waveform)
/// - 2..: Reserved
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct TraceFlags(pub u64);

impl TraceFlags {
    pub const INSTRUCTION: u64 = 1 << 0;
    pub const WAVETRACE: u64 = 1 << 1;

    #[inline(always)]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    #[inline(always)]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Bit 0: instruction trace
    #[inline(always)]
    pub const fn instruction(flags: u64) -> bool {
        (flags & Self::INSTRUCTION) != 0
    }

    /// Bit 1: waveform trace
    #[inline(always)]
    pub const fn wavetrace(flags: u64) -> bool {
        (flags & Self::WAVETRACE) != 0
    }

    #[inline(always)]
    pub fn set_instruction(&mut self, enable: bool) {
        if enable {
            self.0 |= Self::INSTRUCTION;
        } else {
            self.0 &= !Self::INSTRUCTION;
        }
    }

    #[inline(always)]
    pub fn set_wavetrace(&mut self, enable: bool) {
        if enable {
            self.0 |= Self::WAVETRACE;
        } else {
            self.0 &= !Self::WAVETRACE;
        }
    }
}

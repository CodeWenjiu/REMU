//! V extension configuration: FP level (x/f/d), ELEN, VLENB, and VectorCsrState. One source of truth for V options.

use crate::isa::reg::{VectorCsrFields, VectorCsrState, VrState};

/// Unified CSR-related configuration (vector CSR state, etc.).
pub trait CsrConfig: 'static + Copy {
    /// Vector CSR state type (e.g. `()` when no V, [`VectorCsrFields`] when V is present).
    type VectorCsrState: VectorCsrState;
}

/// Configuration for the V (vector) extension. When `VLENB == 0`, the extension is disabled.
/// Implements [`CsrConfig`] so it can be used as the CSR config in state.
pub trait VExtensionConfig: CsrConfig {
    /// Float level: 0 = int only (Zve32x), 1 = float32 (Zve32f), 2 = float64 (Zve64d, etc.).
    const FP_LEVEL: u8 = 0;

    /// Element width in bits: 32 or 64.
    const ELEN: usize = 32;

    /// VLEN/8 in bytes. 0 = no V extension; otherwise the vlenb CSR value.
    const VLENB: u32 = 0;

    /// Vector register file (v0–v31). `()` when no V, `[[u8; VLENB]; 32]` when V is present.
    type VrState: VrState;
}

/// No V extension.
#[derive(Debug, Clone, Copy)]
pub struct NoV;
impl CsrConfig for NoV {
    type VectorCsrState = ();
}
impl VExtensionConfig for NoV {
    const FP_LEVEL: u8 = 0;
    const ELEN: usize = 32;
    const VLENB: u32 = 0;
    type VrState = ();
}

/// Zve32x + Zvl128b: int only, ELEN=32, VLEN=128 bits => VLENB=16.
#[derive(Debug, Clone, Copy)]
pub struct Zve32xZvl128b;
impl CsrConfig for Zve32xZvl128b {
    type VectorCsrState = VectorCsrFields<16>;
}
impl VExtensionConfig for Zve32xZvl128b {
    const FP_LEVEL: u8 = 0;
    const ELEN: usize = 32;
    const VLENB: u32 = 16;
    type VrState = [[u8; 16]; 32];
}

/// Zve32f: float32 support, ELEN=32. Add Zvl* as needed (e.g. Zvl128b => VLENB=16).
#[derive(Debug, Clone, Copy)]
pub struct Zve32fZvl128b;
impl CsrConfig for Zve32fZvl128b {
    type VectorCsrState = VectorCsrFields<16>;
}
impl VExtensionConfig for Zve32fZvl128b {
    const FP_LEVEL: u8 = 1;
    const ELEN: usize = 32;
    const VLENB: u32 = 16;
    type VrState = [[u8; 16]; 32];
}

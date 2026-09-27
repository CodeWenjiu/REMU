//! ISA type definitions — one table, two generated layers.
//!
//! [`Isa`] is the **value-level** ISA identity (`xlen()`, `misa()`, `isa_str()`,
//! `has_m()`): what non-generic consumers (CLI, xtask, platform checks, logging)
//! need. The zero-sized marker types (`RV32I`, …) are the **type-level** identity
//! used as the compile-time ISA of a `State`; their `XLEN`/`FprState`/`VConfig`
//! decide the state layout and cannot be derived from a runtime value.
//!
//! Both are generated from the same table row, so every attribute is written
//! exactly once. To add an ISA variant, add a row to `for_each_isa!` below.

#![allow(non_camel_case_types)]

/// XLEN bits -> machine word type.
macro_rules! xlen_type {
    (32) => {
        u32
    };
    (64) => {
        u64
    };
}

/// FPR state type -> F extension presence.
macro_rules! fpr_present {
    (()) => {
        false
    };
    ($_t:ty) => {
        true
    };
}

// ── Table ──
// Row: Name { xlen: 32|64, fpr: FPR state type, v: V config type, misa: low 32
//             bits of the `misa` CSR, isa_str: canonical ISA string }
//
// Platform support (which backend accepts this ISA) is deliberately *not* here:
// each backend owns its own matrix (`RemuIsaKind` / `NzeaIsaKind`).

#[macro_export]
macro_rules! for_each_isa {
    ($cb:ident) => {
        $cb!(
            RV32I {
                xlen: 32,
                fpr: (),
                v: $crate::isa::extension_v::NoV,
                misa: 0x4000_0100,
                isa_str: "rv32i",
            },
            RV32IM {
                xlen: 32,
                fpr: (),
                v: $crate::isa::extension_v::NoV,
                misa: 0x4000_1100,
                isa_str: "rv32im",
            },
            RV32I_zve32x_zvl128b {
                xlen: 32,
                fpr: (),
                v: $crate::isa::extension_v::Zve32xZvl128b,
                misa: 0x4000_0100,
                isa_str: "rv32i_zve32x_zvl128b",
            },
            RV32IM_zve32x_zvl128b {
                xlen: 32,
                fpr: (),
                v: $crate::isa::extension_v::Zve32xZvl128b,
                misa: 0x4000_1100,
                isa_str: "rv32im_zve32x_zvl128b",
            },
            RV64I {
                xlen: 64,
                fpr: (),
                v: $crate::isa::extension_v::NoV,
                misa: 0x0000_0100,
                isa_str: "rv64i",
            },
            RV64IM {
                xlen: 64,
                fpr: (),
                v: $crate::isa::extension_v::NoV,
                misa: 0x0000_1100,
                isa_str: "rv64im",
            },
        );
    };
}

// ── Generator ──

macro_rules! gen_from_table {
    ($( $N:ident {
        xlen: $xlen:tt, fpr: $fpr:tt, v: $v:ty,
        misa: $misa:expr, isa_str: $s:literal $(,)?
    } ),* $(,)?) => {
        /// Value-level ISA identity: attributes readable at runtime, for
        /// consumers that do not carry the ISA as a type parameter.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Isa {
            $( $N ),*
        }

        impl Isa {
            /// Every variant, in table order.
            pub const ALL: &'static [Isa] = &[ $( Self::$N ),* ];

            /// XLEN in bits (32 or 64).
            pub const fn xlen(self) -> u32 {
                match self { $( Self::$N => $xlen ),* }
            }

            /// Low 32 bits of the `misa` CSR. The MXL field lives in bits
            /// [63:62] and is derived from [`xlen`](Self::xlen) (32 -> 01,
            /// 64 -> 10), never stored here.
            pub const fn misa(self) -> u32 {
                match self { $( Self::$N => $misa ),* }
            }

            /// Canonical ISA string (e.g. `rv32im`).
            pub const fn isa_str(self) -> &'static str {
                match self { $( Self::$N => $s ),* }
            }

            /// M extension present (`misa` bit 12).
            pub const fn has_m(self) -> bool {
                self.misa() & (1 << 12) != 0
            }
        }

        $(
            #[derive(Clone, Copy)]
            pub struct $N;

            impl $crate::isa::RvIsa for $N {
                type XLEN = xlen_type!($xlen);
                type PcState = $crate::isa::reg::PcState<Self::XLEN>;
                type GprState = $crate::isa::reg::GprState<Self::XLEN>;
                type FprState = $fpr;
                type VConfig = $v;
                const ISA: Isa = Isa::$N;
                const ISA_STR: &'static str = Self::ISA.isa_str();
                const MISA: u32 = Self::ISA.misa();
                const HAS_M: bool = Self::ISA.has_m();
                const HAS_F: bool = fpr_present!($fpr);
            }
        )*
    };
}

for_each_isa!(gen_from_table);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::RvIsa;

    #[test]
    fn value_level_attributes() {
        assert_eq!(Isa::RV32I.xlen(), 32);
        assert_eq!(Isa::RV64IM.xlen(), 64);
        assert_eq!(Isa::RV32IM.isa_str(), "rv32im");
        assert_eq!(
            Isa::RV32IM_zve32x_zvl128b.isa_str(),
            "rv32im_zve32x_zvl128b"
        );
        assert!(Isa::RV32IM.has_m());
        assert!(!Isa::RV32I.has_m());
    }

    #[test]
    fn marker_types_match_the_enum() {
        assert_eq!(<RV64IM as RvIsa>::MISA, Isa::RV64IM.misa());
        assert_eq!(<RV32I as RvIsa>::ISA_STR, Isa::RV32I.isa_str());
        const { assert!(<RV32IM as RvIsa>::HAS_M) };
        const { assert!(!<RV64I as RvIsa>::HAS_M) };
        const { assert!(!<RV32I as RvIsa>::HAS_F) };
    }

    #[test]
    fn all_variants_carry_valid_xlen() {
        for isa in Isa::ALL {
            assert!(matches!(isa.xlen(), 32 | 64), "{isa:?}");
        }
    }
}

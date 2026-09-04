//! NES cartridge loaded from ROMs embedded at compile time.
//!
//! Parses the iNES header and serves PRG/CHR banks to the mapper via the
//! `Cartridge` trait. Backed by `include_bytes!`, so no heap is needed.
//!
//! Handles both CHR-ROM carts (CHR data in the file) and CHR-RAM carts
//! (CHR data written by the console at runtime; the file has none).

use runes_core::cartridge::{BankType, Cartridge, MirrorType};
use runes_core::utils::{Read, Write};

/// A ROM embedded in the binary: display title + raw bytes + optional subtitle.
pub(crate) struct EmbeddedRom {
    /// Name shown in the launcher menu.
    pub(crate) name: &'static str,
    /// The raw iNES file bytes.
    pub(crate) data: &'static [u8],
}

/// All embedded ROMs, in menu order. Each is a legal homebrew ROM.
///
/// Note: `alter_ego.nes` is intentionally *not* listed — it crashes the
/// vendored `runes_core` emulator (segfault shortly after boot, even without
/// the Slint menu), so it would only ever fail if selected.
pub(crate) const ROMS: &[EmbeddedRom] = &[
    EmbeddedRom {
        name: "Chase",
        data: include_bytes!("../roms/chase.nes"),
    },
    EmbeddedRom {
        name: "Lan Master",
        data: include_bytes!("../roms/lan_master.nes"),
    },
    EmbeddedRom {
        name: "Lawn Mower",
        data: include_bytes!("../roms/lawn_mower.nes"),
    },
];

/// The display name of ROM `idx` (for the launcher menu).
pub(crate) fn rom_name(idx: usize) -> &'static str {
    ROMS[idx].name
}

/// Number of embedded ROMs.
pub(crate) fn rom_count() -> usize {
    ROMS.len()
}

/// Static battery-backed SRAM (2 KiB, per iNES convention).
static mut SRAM: [u8; 0x2000] = [0; 0x2000];
/// Static CHR RAM (8 KiB) used when the cartridge has no CHR ROM.
static mut CHR_RAM: [u8; 0x2000] = [0; 0x2000];

/// A cartridge backed by static ROM slices (no allocation).
pub(crate) struct StaticCart {
    prg_rom: &'static [u8],
    /// CHR ROM bytes from the file (empty for CHR-RAM carts).
    chr_rom: &'static [u8],
    mirror_type: MirrorType,
}

impl StaticCart {
    pub(crate) fn new(
        prg_rom: &'static [u8],
        chr_rom: &'static [u8],
        mirror_type: MirrorType,
    ) -> Self {
        StaticCart {
            prg_rom,
            chr_rom,
            mirror_type,
        }
    }

    /// The SRAM slice (mutable, static).
    fn sram_mut(&self) -> &'static mut [u8] {
        // SAFETY: `SRAM` is a `static mut` only ever touched through this
        // helper, which hands out disjoint mutable borrows one at a time.
        unsafe { &mut *core::ptr::addr_of_mut!(SRAM) }
    }

    /// The CHR RAM slice (mutable, static), used for CHR-RAM carts.
    fn chr_ram_mut(&self) -> &'static mut [u8] {
        // SAFETY: as above for SRAM.
        unsafe { &mut *core::ptr::addr_of_mut!(CHR_RAM) }
    }

    fn chr_bank_mut(&self) -> &'static mut [u8] {
        if self.chr_rom.is_empty() {
            self.chr_ram_mut()
        } else {
            // SAFETY: cast away immutability; the ROM bytes are never written
            // in practice for CHR-ROM carts (PPU only reads pattern tables).
            unsafe {
                core::slice::from_raw_parts_mut(
                    self.chr_rom.as_ptr() as *mut u8,
                    self.chr_rom.len(),
                )
            }
        }
    }
}

impl Cartridge for StaticCart {
    fn get_size(&self, kind: BankType) -> usize {
        match kind {
            BankType::PrgRom => self.prg_rom.len(),
            // CHR-ROM carts report the ROM size; CHR-RAM carts report 8 KiB.
            BankType::ChrRom => {
                if self.chr_rom.is_empty() {
                    0x2000
                } else {
                    self.chr_rom.len()
                }
            }
            BankType::Sram => 0x2000,
        }
    }

    fn get_bank<'a>(&self, base: usize, size: usize, kind: BankType) -> &'a [u8] {
        // SAFETY: the ROM/SRAM/CHR-RAM slices live for 'static; we reborrow the
        // requested range. Callers (mappers) guarantee base+size is in bounds.
        unsafe {
            let slice: &[u8] = match kind {
                BankType::PrgRom => self.prg_rom,
                BankType::ChrRom => {
                    if self.chr_rom.is_empty() {
                        &*core::ptr::addr_of!(CHR_RAM)
                    } else {
                        self.chr_rom
                    }
                }
                BankType::Sram => &*core::ptr::addr_of!(SRAM),
            };
            &*(&slice[base..base + size] as *const [u8])
        }
    }

    fn get_bank_mut<'a>(&mut self, base: usize, size: usize, kind: BankType) -> &'a mut [u8] {
        // SAFETY: the returned borrow is disjoint from any other (mappers use
        // one bank at a time); SRAM/CHR-RAM are statics owned exclusively here.
        unsafe {
            let slice: &mut [u8] = match kind {
                BankType::PrgRom => self.sram_mut(), // never actually written for PRG ROM
                BankType::ChrRom => self.chr_bank_mut(),
                BankType::Sram => self.sram_mut(),
            };
            &mut *(&mut slice[base..base + size] as *mut [u8])
        }
    }

    fn get_mirror_type(&self) -> MirrorType {
        self.mirror_type
    }

    fn set_mirror_type(&mut self, mt: MirrorType) {
        self.mirror_type = mt;
    }

    fn load(&mut self, _reader: &mut dyn Read) -> bool {
        false
    }
    fn save(&self, _writer: &mut dyn Write) -> bool {
        false
    }
    fn load_sram(&mut self, _reader: &mut dyn Read) -> bool {
        false
    }
    fn save_sram(&self, _writer: &mut dyn Write) -> bool {
        false
    }
}

/// Parse an embedded iNES ROM into a `StaticCart`.
pub(crate) fn load_embedded_cart(idx: usize) -> StaticCart {
    let rom = ROMS[idx].data;
    let header = &rom[0..16];
    debug_assert_eq!(&header[0..4], b"NES\x1a");
    let prg_nbanks = header[4] as usize;
    let chr_nbanks = header[5] as usize;
    let flags6 = header[6];

    let trainer_len = if flags6 & 0x04 != 0 { 512 } else { 0 };
    let prg_len = prg_nbanks * 0x4000;
    let chr_len = chr_nbanks * 0x2000;

    let prg_start = 16 + trainer_len;
    let prg_rom = &rom[prg_start..prg_start + prg_len];
    // CHR-ROM carts have CHR data after PRG; CHR-RAM carts have none.
    let chr_rom = if chr_len > 0 {
        &rom[prg_start + prg_len..prg_start + prg_len + chr_len]
    } else {
        &rom[0..0]
    };

    let mirror_type = if flags6 & 1 != 0 {
        MirrorType::Vertical
    } else {
        MirrorType::Horizontal
    };

    StaticCart::new(prg_rom, chr_rom, mirror_type)
}

/// The mapper number from the iNES header of ROM `idx`.
pub(crate) fn embedded_mapper_id(idx: usize) -> u8 {
    let rom = ROMS[idx].data;
    let flags6 = rom[6];
    let flags7 = rom[7];
    (flags7 & 0xf0) | (flags6 >> 4)
}

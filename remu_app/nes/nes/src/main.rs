#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

//! A minimal NES emulator running on remu, driven by the vendored `runes_core`.
//!
//! The emulated console (CPU/PPU/APU + mapper) is clocked instruction by
//! instruction; the PPU's `Screen` writes into the display framebuffer, and the
//! keyboard device feeds the NES joypad. Audio is currently a silent stub.

mod cart;
mod input;
mod screen;

use remu_hal::{
    Box, FmtWrite, MTIME_TICK_HZ, Uart16550, display_alive, exit_success, fb_base, read_disp_size,
    read_mtime,
};
use runes_core::apu::{APU, Speaker};
use runes_core::controller::stdctl;
use runes_core::mapper::{self, Mapper, RefMapper};
use runes_core::memory::{CPUMemory, PPUMemory};
use runes_core::mos6502;
use runes_core::ppu::PPU;

/// NES runs at ~60.1 fps; we throttle to ~60 fps via mtime.
const FRAME_MS: u64 = 16;

/// Silent audio sink: we don't have an audio device yet, so APU samples are
/// dropped.
struct SilentSpeaker;
impl Speaker for SilentSpeaker {
    fn queue(&mut self, _sample: i16) {}
}

#[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();
    let _ = writeln!(uart, "nes: booting (runes_core)");

    // ── Cartridge + mapper ──
    let cart = cart::load_embedded_cart();
    let mapper_id = cart::embedded_mapper_id();
    let _ = writeln!(uart, "nes: mapper {mapper_id}");
    let mut mapper_box: Box<dyn Mapper> = match mapper_id {
        0 | 2 => Box::new(mapper::Mapper2::new(cart)),
        1 => Box::new(mapper::Mapper1::new(cart)),
        4 => Box::new(mapper::Mapper4::new(cart)),
        _ => {
            let _ = writeln!(uart, "nes: unsupported mapper {mapper_id}");
            exit_success();
        }
    };
    let mapper = RefMapper::new(mapper_box.as_mut() as &mut dyn Mapper);

    // ── Controller (keyboard → joypad), silent audio, screen ──
    let poller = input::KeyboardPoller::new();
    let ctl1 = stdctl::Joystick::new(&poller);
    let mut speaker = SilentSpeaker;
    let mut screen = {
        let fb = fb_base() as *mut u32;
        let disp = read_disp_size();
        screen::NesScreen::new(fb, disp.width, disp.height)
    };

    // ── Console: CPU / PPU / APU wired through the bus ──
    let mut cpu = mos6502::CPU::new(CPUMemory::new(&mapper, Some(&ctl1), None));
    let mut ppu = PPU::new(PPUMemory::new(&mapper), &mut screen);
    let mut apu = APU::new(&mut speaker);
    let cpu_ptr = &mut cpu as *mut mos6502::CPU;
    cpu.mem.bus.attach(cpu_ptr, &mut ppu, &mut apu);
    cpu.powerup();

    let t0 = read_mtime();
    let mut last = 0u64;
    let mut fps_last = 0u64;
    let mut fps = 0u32;

    loop {
        // Emulate CPU/PPU continuously (like the reference demo). The PPU calls
        // `Screen::frame` at each vblank; we detect that as the frame boundary.
        let mut guard = 0u32;
        let mut frame_rendered = false;
        while !frame_rendered && guard < 300_000 {
            while cpu.cycle > 0 {
                cpu.mem.bus.tick();
            }
            cpu.step();
            guard += 1;
            // SAFETY: single-threaded; Screen::frame sets this on vblank.
            if unsafe { screen::FRAME_DONE } {
                unsafe { screen::FRAME_DONE = false };
                frame_rendered = true;
            }
        }
        poller.update();

        // Throttle to ~60 fps.
        let now = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ as u64;
        if now < last + FRAME_MS {
            continue;
        }
        last = now;

        // Report FPS once a second so we can confirm the render loop runs.
        fps = fps.wrapping_add(1);

        if now - fps_last >= 1000 {
            // SAFETY: single-threaded; only this loop reads/resets the counter.
            let puts = unsafe {
                let p = screen::PUTS_PER_FRAME;
                screen::PUTS_PER_FRAME = 0;
                p
            };
            let _ = writeln!(
                uart,
                "nes: {} fps (pc={:#06x}, puts/s={}, sl={})",
                fps,
                cpu.get_pc(),
                puts,
                ppu.scanline
            );
            fps_last = now;
            fps = 0;
        }

        // Detect window close.
        if !display_alive() {
            let _ = writeln!(uart, "nes: window closed, exiting");
            exit_success();
        }
    }
}

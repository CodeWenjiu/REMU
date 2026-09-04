#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

//! A NES emulator running on remu, driven by the vendored `runes_core`, with a
//! Slint launcher menu for selecting embedded ROMs.
//!
//! The launcher runs in Slint (rendered by remu_hal_slint into the display
//! framebuffer). Once a game is launched, control transfers to the NES loop,
//! which renders directly into the framebuffer via `NesScreen` — Slint is
//! *not* pumped during gameplay, so the menu costs nothing in-game. Press
//! Escape at any time to return to the menu.

extern crate alloc;

mod cart;
mod input;
mod screen;

use alloc::rc::Rc;
use core::cell::Cell;

use remu_hal::{
    Box, FmtWrite, MTIME_TICK_HZ, Uart16550, display_alive, exit_success, fb_base, read_disp_size,
    read_key_kind, read_mtime,
};
use remu_hal_slint::SlintApp;
use runes_core::apu::{APU, Speaker};
use runes_core::controller::stdctl;
use runes_core::mapper::{self, Mapper, RefMapper};
use runes_core::memory::{CPUMemory, PPUMemory};
use runes_core::mos6502;
use runes_core::ppu::PPU;
use slint::ComponentHandle as _;

/// NES runs at ~60.1 fps; we throttle to ~60 fps via mtime.
const FRAME_MS: u64 = 16;

/// Silent audio sink: we don't have an audio device yet, so APU samples are
/// dropped.
struct SilentSpeaker;
impl Speaker for SilentSpeaker {
    fn queue(&mut self, _sample: i16) {}
}

/// A single emulated game session. Returns when the user quits to the menu.
fn run_game(idx: usize, uart: &mut Uart16550) {
    let _ = writeln!(uart, "nes: launching {}", cart::rom_name(idx));

    // ── Cartridge + mapper ──
    let cart = cart::load_embedded_cart(idx);
    let mapper_id = cart::embedded_mapper_id(idx);
    let _ = writeln!(uart, "nes: mapper {mapper_id}");
    let mut mapper_box: Box<dyn Mapper> = match mapper_id {
        0 | 2 => Box::new(mapper::Mapper2::new(cart)),
        1 => Box::new(mapper::Mapper1::new(cart)),
        4 => Box::new(mapper::Mapper4::new(cart)),
        _ => {
            let _ = writeln!(uart, "nes: unsupported mapper {mapper_id}");
            return;
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
    // Audio is discarded (SilentSpeaker); tell the APU so it can skip the
    // audio-only channel timers and mixing, while keeping frame-counter and
    // DMC IRQ timing exact.
    apu.silent = true;
    let cpu_ptr = &mut cpu as *mut mos6502::CPU;
    cpu.mem.bus.attach(cpu_ptr, &mut ppu, &mut apu);
    cpu.powerup();

    let t0 = read_mtime();
    let mut last = 0u64;
    let mut fps_last = 0u64;
    let mut fps = 0u32;
    // Escape edge detection: only quit on a fresh press, not while held.
    let mut esc_was_down = false;

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

        // Quit to menu on a fresh Escape press. `read_key_*` is a snapshot of
        // the latest event; on the host the down edge is almost never observed
        // (press+release land between polls), so detect by key-kind change
        // instead of requiring `down`. Both host and embedded report Escape via
        // `KeyKind::Escape`.
        let esc_pressed = read_key_kind() == remu_hal::KeyKind::Escape;
        if esc_pressed && !esc_was_down {
            let _ = writeln!(uart, "nes: quit to menu");
            return;
        }
        esc_was_down = esc_pressed;

        // Throttle to ~60 fps. Spin until the next frame boundary, so each
        // 16 ms real-time window contains exactly one emulated frame. (Busy-
        // waiting rather than `continue` back to the top: `continue` would
        // immediately re-emulate a frame, over-driving game speed when frames
        // emulate faster than real time.)
        let mut now = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ as u64;
        while now < last + FRAME_MS {
            now = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ as u64;
        }
        last = now;

        // Report FPS once a second so we can confirm the render loop runs.
        fps = fps.wrapping_add(1);
        if now - fps_last >= 1000 {
            let _ = writeln!(
                uart,
                "nes: {} fps (pc={:#06x}, sl={})",
                fps,
                cpu.get_pc(),
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

#[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();
    let _ = writeln!(uart, "nes: booting (runes_core + slint menu)");

    // Install the Slint platform once (process-wide). The launcher menu is
    // rendered through it; during gameplay we don't pump it, so it costs
    // nothing.
    let mut app = SlintApp::init();

    // ── Launcher menu (Slint) ──
    let menu = ui::RomMenu::new().expect("create RomMenu");
    menu.window().set_size(slint::PhysicalSize::new(
        remu_hal::read_disp_w().max(1) as u32,
        remu_hal::read_disp_h().max(1) as u32,
    ));
    // Fill the ROM list with the embedded titles.
    let rom_names: remu_hal::Vec<slint::SharedString> = (0..cart::rom_count())
        .map(|i| cart::rom_name(i).into())
        .collect();
    menu.set_roms(slint::ModelRc::from(Rc::new(slint::VecModel::from(
        rom_names,
    ))));

    // The pending game index, set by the `launch` callback and consumed by the
    // menu loop. Slint callbacks run synchronously during `app.update()`, and
    // this is shared so the callback can write it while the loop reads it.
    let chosen = Rc::new(Cell::new(None));
    let chosen_cb = Rc::clone(&chosen);
    let menu_handle = menu.as_weak();
    menu.on_launch(move |idx| {
        if let Some(handle) = menu_handle.upgrade() {
            // Keep the list's current selection in sync (in-out property).
            handle.set_selected_index(idx);
        }
        chosen_cb.set(Some(idx as usize));
    });

    let t0 = read_mtime();
    let mut last = 0u64;

    // Menu loop: pump Slint until a ROM is chosen (or the window is closed).
    fn wait_for_selection(
        app: &mut SlintApp,
        chosen: &Rc<Cell<Option<usize>>>,
        uart: &mut Uart16550,
        t0: u64,
        last: &mut u64,
    ) -> usize {
        loop {
            // Exit if the window is closed (closing the window quits the app).
            if !display_alive() {
                let _ = writeln!(uart, "nes: exiting");
                exit_success();
            }
            app.update();
            // Throttle to ~60 fps so the menu doesn't spin the CPU.
            let mut now = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ as u64;
            while now < *last + FRAME_MS {
                now = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ as u64;
            }
            *last = now;
            if let Some(idx) = chosen.get() {
                chosen.set(None);
                return idx;
            }
        }
    }

    // First ROM selection, then run games forever. After each game returns
    // (user pressed Escape), the Slint renderer repaints the whole framebuffer
    // on the next `update`, covering the game's last frame.
    loop {
        let idx = wait_for_selection(&mut app, &chosen, &mut uart, t0, &mut last);
        run_game(idx, &mut uart);
        // The game read the raw keyboard device directly (Escape to quit);
        // forget that state so it isn't re-delivered to the menu on the next
        // `app.update()`.
        app.resync_key_state();
    }
}

// Generated Slint UI for the launcher menu.
#[allow(unreachable_pub)]
mod ui {
    slint::include_modules!();
}

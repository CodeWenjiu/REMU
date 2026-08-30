#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

use remu_hal::{FmtWrite, Uart16550, exit_success};

/// Framebuffer base address (matches the display device's declared region).
const FB_BASE: usize = 0x8900_0000;
/// Framebuffer dimensions (must match the display device).
const FB_WIDTH: usize = 800;
const FB_HEIGHT: usize = 600;
const FB_BPP: usize = 4; // RGBA

#[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();

    // Draw into the framebuffer directly (software writes to the memory-mapped
    // framebuffer region, exactly as the display device expects).
    let fb = FB_BASE as *mut u8;

    // 1. Horizontal gradient (R from 0..255 across, G/B fixed).
    for y in 0..FB_HEIGHT {
        for x in 0..FB_WIDTH {
            let i = (y * FB_WIDTH + x) * FB_BPP;
            let r = (x * 255 / (FB_WIDTH - 1)) as u8;
            let g = (y * 255 / (FB_HEIGHT - 1)) as u8;
            let b = 128u8;
            unsafe {
                *fb.add(i) = r;
                *fb.add(i + 1) = g;
                *fb.add(i + 2) = b;
                *fb.add(i + 3) = 255; // opaque
            }
        }
    }

    // 2. Overlay a colored square in the middle (e.g. 200x200).
    let square = 200usize;
    let x0 = (FB_WIDTH - square) / 2;
    let y0 = (FB_HEIGHT - square) / 2;
    for y in y0..y0 + square {
        for x in x0..x0 + square {
            let i = (y * FB_WIDTH + x) * FB_BPP;
            unsafe {
                *fb.add(i) = 255; // R
                *fb.add(i + 1) = 0; // G
                *fb.add(i + 2) = 0; // B
                *fb.add(i + 3) = 255; // A
            }
        }
    }

    let _ = writeln!(
        uart,
        "display_test: wrote gradient + red square to framebuffer"
    );

    // Keep running for a few seconds so the window has time to render the frame
    // before the device is torn down on exit. mtime ticks at MTIME_TICK_HZ.
    let hold_ticks = 5 * remu_hal::MTIME_TICK_HZ;
    let start = remu_hal::read_mtime();
    while remu_hal::read_mtime().wrapping_sub(start) < hold_ticks {
        core::hint::spin_loop();
    }

    exit_success()
}

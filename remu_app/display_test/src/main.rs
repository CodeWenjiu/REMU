#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

use remu_hal::{FmtWrite, Uart16550, read_mtime};

/// Framebuffer base address (matches the display device's declared region).
const FB_BASE: usize = 0x8900_0000;
/// Framebuffer dimensions (must match the display device).
const FB_WIDTH: usize = 800;
const FB_HEIGHT: usize = 600;
/// Display device MMIO base (matches `--dev display@0x88000000`).
const DISPLAY_BASE: usize = 0x8800_0000;
/// Control register offset: writing here signals "frame finished".
const CTRL_FRAME_DONE: usize = 4;
/// Number of frames rendered per second (driven by mtime).
const FPS: u64 = 60;
/// mtime ticks per frame.
const TICKS_PER_FRAME: u64 = remu_hal::MTIME_TICK_HZ / FPS;

/// Signal to the display device that the current frame is complete, so the
/// render thread blits it at a frame boundary (prevents tearing).
#[inline(always)]
fn frame_done() {
    unsafe {
        core::ptr::write_volatile((DISPLAY_BASE + CTRL_FRAME_DONE) as *mut u32, 1);
    }
}

/// 0RGB pixel: bytes [B, G, R, 0] as a little-endian u32.
#[inline(always)]
fn pixel(r: u8, g: u8, b: u8) -> u32 {
    u32::from_le_bytes([b, g, r, 0])
}

/// Write a single 0RGB pixel to the framebuffer.
#[inline(always)]
fn put_pixel(fb: *mut u32, x: usize, y: usize, color: u32) {
    if x < FB_WIDTH && y < FB_HEIGHT {
        unsafe {
            *fb.add(y * FB_WIDTH + x) = color;
        }
    }
}

/// 8-bit cosine via a coarse 16-sample table with linear interpolation (no std).
static COS16: [i16; 16] = [
    128, 121, 99, 66, 24, -17, -55, -86, -106, -111, -99, -71, -33, 10, 51, 88,
];

/// Approximate `cos(2π * k / 256)` * 128 (i.e. returns [-128, 127]).
#[inline(always)]
fn cos8(k: usize) -> i32 {
    let k = k & 255;
    let i = k / 16;
    let f = (k % 16) as i32;
    let a = COS16[i] as i32;
    let b = COS16[(i + 1) & 15] as i32;
    (a * (16 - f) + b * f) / 16
}

/// Approximate `sin(2π * k / 256)` * 128.
#[inline(always)]
fn sin8(k: usize) -> i32 {
    cos8(k.wrapping_sub(64))
}

/// Draw a solid (flat-color) circle. Used to erase a ball back to the
/// background after it moves.
#[inline]
fn fill_circle(fb: *mut u32, cx: isize, cy: isize, radius: isize, color: u32) {
    let r2 = radius * radius;
    let (x0, x1) = (
        (cx - radius).max(0) as usize,
        (cx + radius).min(FB_WIDTH as isize - 1) as usize,
    );
    let (y0, y1) = (
        (cy - radius).max(0) as usize,
        (cy + radius).min(FB_HEIGHT as isize - 1) as usize,
    );
    for y in y0..=y1 {
        let dy = y as isize - cy;
        for x in x0..=x1 {
            let dx = x as isize - cx;
            if dx * dx + dy * dy <= r2 {
                put_pixel(fb, x, y, color);
            }
        }
    }
}

/// Draw a filled circle with a soft gradient (bright at center, fading out).
#[inline]
fn draw_ball(fb: *mut u32, cx: isize, cy: isize, radius: isize, hue: u8) {
    let r2 = radius * radius;
    let (x0, x1) = (
        (cx - radius).max(0) as usize,
        (cx + radius).min(FB_WIDTH as isize - 1) as usize,
    );
    let (y0, y1) = (
        (cy - radius).max(0) as usize,
        (cy + radius).min(FB_HEIGHT as isize - 1) as usize,
    );
    for y in y0..=y1 {
        let dy = y as isize - cy;
        for x in x0..=x1 {
            let dx = x as isize - cx;
            let d2 = dx * dx + dy * dy;
            if d2 <= r2 {
                // Radial fade using an integer distance approximation (no sqrt):
                // normalize by the bounding square, bright center → dim edge.
                let norm = (d2 * 1024 / r2) as usize; // 0..=1024 (1.0 at edge)
                let t = 255usize.saturating_sub(norm * 255 / 1024);
                let (r, g, b) = hue_rgb(hue, t as u8);
                put_pixel(fb, x, y, pixel(r, g, b));
            }
        }
    }
}

/// HSV→RGB, hue 0..255 (wraps a full wheel), value/saturation fixed-ish.
#[inline(always)]
fn hue_rgb(hue: u8, v: u8) -> (u8, u8, u8) {
    let h = hue as u32 * 360 / 256; // 0..359
    let s = 200u32; // saturation
    let vv = v as u32;
    let region = h / 60;
    let f = h % 60;
    let p = vv * (255 - s) / 255;
    let q = vv * (255 - s * f / 60) / 255;
    let t = vv * (255 - s * (60 - f) / 60) / 255;
    match region {
        0 => (vv as u8, t as u8, p as u8),
        1 => (q as u8, vv as u8, p as u8),
        2 => (p as u8, vv as u8, t as u8),
        3 => (p as u8, q as u8, vv as u8),
        4 => (t as u8, p as u8, vv as u8),
        _ => (vv as u8, p as u8, q as u8),
    }
}

/// Number of trail balls drawn each frame (including the head).
const TRAIL: usize = 6;

#[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();

    let fb = FB_BASE as *mut u32;

    let _ = writeln!(uart, "display_test: rainbow orbit animation (0RGB)");

    // ── Static background: draw once, never redrawn. A dark diagonal gradient
    //    so the bright ball pops. Keeps per-frame drawing cost tiny. ──
    for y in 0..FB_HEIGHT {
        for x in 0..FB_WIDTH {
            let hue = (x as usize + y as usize) & 255;
            let (r, g, b) = hue_rgb(hue as u8, 40);
            put_pixel(fb, x, y, pixel(r, g, b));
        }
    }

    let mut frame = 0u32;
    let mut prev_frame_end = read_mtime();
    // Positions of each trail ball in the previous frame (for erasing).
    let mut prev_pos = [(0isize, 0isize); TRAIL];

    loop {
        // ── Advance to the next frame boundary (60 fps). ──
        let target = prev_frame_end.wrapping_add(TICKS_PER_FRAME);
        while read_mtime() < target {
            core::hint::spin_loop();
        }
        prev_frame_end = target;

        let t = frame as u32;
        let orbit_radius = 220isize;
        let cx = (FB_WIDTH as isize) / 2;
        let cy = (FB_HEIGHT as isize) / 2;
        let ang = (t as usize * 4) & 255; // 256 steps per orbit, speed 4/frame

        // ── Erase the previous frame's balls back to the background color. ──
        for (i, (px, py)) in prev_pos.iter().enumerate() {
            let radius = (28 - i as isize * 3).max(6);
            let (br, bg, bb) = hue_rgb(((*px as usize + *py as usize) & 255) as u8, 40);
            fill_circle(fb, *px, *py, radius, pixel(br, bg, bb));
        }

        // ── Draw the new trail (head ball first). ──
        let mut new_pos = [(0isize, 0isize); TRAIL];
        for trail in 0..TRAIL {
            let back = trail * 8;
            let a = ang.wrapping_sub(back);
            let bx = cx + orbit_radius * cos8(a) as isize / 128;
            let by = cy + orbit_radius * sin8(a) as isize / 128;
            let hue = (t as usize + trail * 24) as u8;
            let radius = (28 - trail as isize * 3).max(6) as isize;
            draw_ball(fb, bx, by, radius, hue);
            new_pos[trail] = (bx, by);
        }
        prev_pos = new_pos;

        frame = frame.wrapping_add(1);

        // ── Signal frame completion: the render thread blits now, at a frame
        //    boundary, so the window never shows a half-drawn frame. ──
        frame_done();
    }
}

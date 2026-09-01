#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

//! A simple, self-adapting RGB plasma shader.
//!
//! Each color channel is an independent smooth sine wave travelling across the
//! screen at a slightly different speed/direction, so the result is a slowly
//! shifting, soft rainbow gradient:
//!
//!   R = wave(u - t)          horizontal band drifting right
//!   G = wave(v - 4t/5)       vertical band drifting up
//!   B = wave((u+v)/2 - 3t/5) diagonal band drifting slowly
//!
//! Rendered in fixed point (no f32) with a 256-entry cosine lookup table. The
//! active display size is read from the display device each frame
//! (`read_disp_w`/`read_disp_h`), so it adapts to any window size without hard
//! assumptions.
//!
//! Mouse interaction:
//!   - cursor X/Y shifts the plasma's phase center (sweep the gradient)
//!   - holding a mouse button speeds the animation up 3×

use remu_hal::{
    FB_WIDTH, FmtWrite, MTIME_TICK_HZ, Uart16550, fb_base, frame_done, put_pixel, read_disp_size,
    read_mouse, read_mtime,
};

/// Render at 1/3 resolution, upscale 3× (cheap enough for a good frame rate).
const UP: usize = 6;

/// Animation speed: full cosine period (one turn) every TURN/TURNS_PER_SEC sec.
const TURNS_PER_SEC: i64 = 24;

/// One full turn = 360° = 2π radians (cos table granularity).
const TURN: i32 = 256;
/// Cos table stores cos(2π k / TURN) * SCALE.
const SCALE: i32 = 1024;

/// 256-entry cosine table: cos(2π k / 256) * SCALE (10-bit fixed point).
static mut COS: [i16; 256] = [0; 256];

fn init_cos(c: &mut [i16; 256]) {
    // Chebyshev recurrence: cos(kθ) = 2·cos(θ)·cos((k-1)θ) − cos((k-2)θ).
    const CD: i64 = 16379; // cos(2π/256) · 16384 ≈ 16379
    let mut c0: i64 = 16384;
    let mut c1: i64 = CD;
    c[0] = (c0 * SCALE as i64 / 16384) as i16;
    c[1] = (c1 * SCALE as i64 / 16384) as i16;
    for k in 2..256 {
        let ck = (2 * CD * c1 / 16384) - c0;
        c0 = c1;
        c1 = ck;
        c[k] = (ck * SCALE as i64 / 16384) as i16;
    }
}

/// Lookup `cos(2π * a / TURN) * SCALE`, `a` any integer (wraps mod TURN).
#[inline(always)]
fn cos_fixed(a: i32) -> i32 {
    let a = a & (TURN - 1);
    // SAFETY: COS initialized once before any use.
    unsafe { COS[a as usize] as i32 }
}

/// Normalize a cosine value into [0, SCALE]: (cos + 1) / 2.
#[inline(always)]
fn wave(a: i32) -> i32 {
    (cos_fixed(a) + SCALE) / 2
}

/// Shade one internal pixel at `(x, y)` of an `iw × ih` render. `t` is the time
/// in turns; `mx_t`/`my_t` are the mouse position mapped into turn offsets;
/// `fast` triples the animation speed. Returns a 0RGB u32.
#[inline(always)]
fn shade(
    x: usize,
    y: usize,
    iw: usize,
    ih: usize,
    t: i32,
    mx_t: i32,
    my_t: i32,
    fast: bool,
) -> u32 {
    // Normalized coordinates in turns: u, v ∈ [0, TURN).
    let u = (x * TURN as usize / iw) as i32;
    let v = (y * TURN as usize / ih) as i32;

    // Mouse shifts each wave's phase → moving the cursor sweeps the gradient.
    let t = if fast { t * 3 } else { t };
    let r = wave(u - t + mx_t);
    let g = wave(v - t * 4 / 5 + my_t);
    let b = wave((u + v) / 2 - t * 3 / 5 + (mx_t + my_t) / 2);

    let cr = (r * 255 / SCALE) as u32;
    let cg = (g * 255 / SCALE) as u32;
    let cb = (b * 255 / SCALE) as u32;
    cb | (cg << 8) | (cr << 16)
}

#[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();

    // SAFETY: single-threaded startup; init before any use.
    unsafe {
        init_cos(&mut *core::ptr::addr_of_mut!(COS));
    }

    let fb = fb_base() as *mut u32;
    let _ = writeln!(uart, "display_test: plasma shader (0RGB)");

    let t0 = read_mtime();

    loop {
        // Time in turns, wrapped to [0, TURN): TURNS_PER_SEC turns per second.
        let elapsed = read_mtime().wrapping_sub(t0);
        let t_turn = ((elapsed as i64 * TURNS_PER_SEC / MTIME_TICK_HZ as i64) % TURN as i64) as i32;

        // ── Self-adapting to the actual window size. ──
        let disp = read_disp_size();
        let disp_w = disp.width.clamp(2, FB_WIDTH);
        let disp_h = disp.height.clamp(2, FB_WIDTH);
        let iw = (disp_w / UP).max(1);
        let ih = (disp_h / UP).max(1);

        // ── Mouse: position → turn offsets, buttons → fast mode. ──
        let mouse = read_mouse();
        let mx = mouse.x.clamp(0, disp_w);
        let my = mouse.y.clamp(0, disp_h);
        let fast = mouse.buttons != 0;
        let mx_t = (mx as i64 * TURN as i64 / disp_w as i64) as i32;
        let my_t = (my as i64 * TURN as i64 / disp_h as i64) as i32;

        // ── Render. ──
        for ry in 0..ih {
            for rx in 0..iw {
                let v = shade(rx, ry, iw, ih, t_turn, mx_t, my_t, fast);
                let (bx, by) = (rx * UP, ry * UP);
                for b in by..by + UP {
                    for a in bx..bx + UP {
                        put_pixel(fb, a, b, v);
                    }
                }
            }
        }

        frame_done();
    }
}

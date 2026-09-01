#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

//! AM video test 的 Rust 移植：N×N 彩色螺旋刷动画。
//!
//! 参考 https://github.com/NJU-ProjectN/am-kernels/blob/master/tests/am-tests/src/tests/video.c
//!
//! - 画布分成 N×N 个纯色块，每帧沿螺旋路径重新填充颜色
//! - 颜色由计数器 `tsc` 决定（b = tsc & 0xff，RGB = (b*6, b*7, b)）
//! - 以 30 FPS 刷新，每秒打印一次 FPS
//!
//! 鼠标交互（方案 A）：
//! - 鼠标位置 → 扩散中心（颜色从鼠标处向外涟漪扩散）
//! - 左键 → 加速（tsc 步进 ×4）；右键 → 放慢（步进减半）
//!
//! 计算量极小（纯整数，无浮点），适合任何平台。

use remu_hal::{
    FB_HEIGHT, FB_WIDTH, FmtWrite, MTIME_TICK_HZ, Uart16550, fb_base, frame_done, put_pixel,
    read_disp_size, read_mouse, read_mtime,
};

/// 画布网格数（video.c 的 N）。
const N: usize = 32;
/// 目标帧率。
const FPS: u32 = 30;

/// 从 `(r, g, b)` 构造 0RGB 像素。
#[inline(always)]
fn pixel(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// 由计数器生成颜色：b = tsc & 0xff，RGB = (b*6, b*7, b)。
#[inline(always)]
fn p(tsc: u32) -> u32 {
    let b = (tsc & 0xff) as u32;
    pixel((b * 6) as u8, (b * 7) as u8, b as u8)
}

/// 画布：N×N 每格一个颜色。
static mut CANVAS: [[u32; N]; N] = [[0; N]; N];
/// 画布：该格是否已被螺旋填充。
static mut USED: [[u8; N]; N] = [[0; N]; N];

/// 从中心向外扩散填充画布，中心由鼠标决定（水波式 BFS，必然覆盖全图）。
///
/// `cx`/`cy` 是扩散中心（网格坐标）；`init` 是起始颜色计数。
fn update(cx: usize, cy: usize, init: u32) {
    // SAFETY: 单线程裸机环境。
    let canvas = unsafe { &mut *core::ptr::addr_of_mut!(CANVAS) };
    let used = unsafe { &mut *core::ptr::addr_of_mut!(USED) };

    for row in used.iter_mut() {
        for c in row.iter_mut() {
            *c = 0;
        }
    }

    // BFS 从中心向四方扩散（四邻域）。固定数组作队列，最多 N*N 格。
    let mut queue = [(0usize, 0usize); N * N];
    let mut head = 0usize;
    let mut tail = 0usize;
    canvas[cy][cx] = p(init);
    used[cy][cx] = 1;
    queue[tail] = (cx, cy);
    tail += 1;
    let mut dist = 0u32;
    while head < tail {
        // 当前层剩余节点数 = tail - head（用层计数控制颜色渐变）。
        let level_end = tail;
        let mut seen = 0u32;
        while head < level_end {
            let (x, y) = queue[head];
            head += 1;
            // 四邻域。
            let dirs = [(0i32, 1i32), (1, 0), (0, -1), (-1, 0)];
            for (dx, dy) in dirs {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < N as i32 && ny >= 0 && ny < N as i32 {
                    let (nx, ny) = (nx as usize, ny as usize);
                    if used[ny][nx] == 0 {
                        used[ny][nx] = 1;
                        // 颜色随层数快速变化（每层一个色阶），让涟漪更细密。
                        canvas[ny][nx] = p(init + dist * 2);
                        queue[tail] = (nx, ny);
                        tail += 1;
                        seen += 1;
                    }
                }
            }
        }
        dist += 1;
        let _ = seen;
    }
}

/// 把画布画到 framebuffer（video.c 的 redraw）。
fn redraw(fb: *mut u32, disp_w: usize, disp_h: usize) {
    // SAFETY: 单线程裸机环境。
    let canvas = unsafe { &*core::ptr::addr_of!(CANVAS) };

    let w = disp_w / N;
    let h = disp_h / N;

    for (y, row) in canvas.iter().enumerate() {
        for (x, &col) in row.iter().enumerate() {
            let (bx, by) = (x * w, y * h);
            for b in by..by + h {
                for a in bx..bx + w {
                    put_pixel(fb, a, b, col);
                }
            }
        }
    }
    // 帧完成信号（video.c 的同步 FBDRAW 尾部）。
    frame_done();
}

#[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();

    let fb = fb_base() as *mut u32;
    let _ = writeln!(uart, "display: AM video spiral test (0RGB)");

    let t0 = read_mtime();
    let mut tsc: u32 = 0;
    let mut last = 0u64;
    let mut fps_last = 0u64;
    let mut fps = 0u32;

    loop {
        // 当前毫秒数。
        let upt = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ as u64;

        if upt - last > (1000 / FPS) as u64 {
            // ── 鼠标交互（方案 A：扩散中心跟随鼠标）──
            let mouse = read_mouse();
            let disp = read_disp_size();
            let disp_w = disp.width.clamp(2, FB_WIDTH);
            let disp_h = disp.height.clamp(2, FB_HEIGHT);
            // 鼠标位置 → 扩散中心（网格坐标）。
            let mx = mouse.x.clamp(0, disp_w);
            let my = mouse.y.clamp(0, disp_h);
            let cx = (mx as u32 * N as u32 / disp_w as u32) as usize;
            let cy = (my as u32 * N as u32 / disp_h as u32) as usize;
            // 颜色相位由 tsc 决定；起始色随相位变化。
            let init = tsc;
            // 左键加速 ×4，右键放慢减半。
            let step = if mouse.buttons & 1 != 0 {
                4u32
            } else if mouse.buttons & 2 != 0 {
                1
            } else {
                2
            };

            tsc = tsc.wrapping_add(step);
            update(cx, cy, init);
            redraw(fb, disp_w, disp_h);
            last = upt;
            fps = fps.wrapping_add(1);
        }
        if upt - fps_last > 1000 {
            let _ = writeln!(uart, "{}: FPS = {}", upt, fps);
            fps_last = upt;
            fps = 0;
        }
    }
}

#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

//! 60 个发光点沿轨道运行的 shader（来自 glslsandbox）。
//!
//! 每个点围绕中心做螺旋运动：x 相位 `θ·t`、y 相位 `θ−t`，颜色随时间
//! 旋转，亮度按 `1/d` 衰减。整个图案像一朵缓慢绽放的旋转光花。
//!
//! 固定点数学（无 f32）+ 256 项 cos 查表，适配无 FPU 的 RISC-V。渲染
//! 分辨率自动跟随窗口大小（`read_disp_size`），并上采样 `UP` 倍。

use remu_hal::{
    FB_WIDTH, FmtWrite, MTIME_TICK_HZ, Uart16550, fb_base, frame_done, put_pixel, read_disp_size,
    read_mtime,
};

/// 渲染分辨率降低因子（上采样倍数）。越小越清晰但越慢。
const UP: usize = 2;
/// 发光点数量（降到 60 以下会变稀疏，大小/范围分布保持不变）。
const N: usize = 60;
/// 原始 shader 的点数（决定 size/θ/颜色节奏的基准）。
const G: usize = 60;
/// Q14 定点（16384 = 1.0）。
const Q: i64 = 16384;
/// 轨道半径 r = 0.6（Q14）。
const R: i64 = 9830;
/// cos 表粒度：一整圈 = TURN。
const TURN: i32 = 256;
/// cos 表比例：cos 值 × SCALE。
const SCALE: i64 = 1024;
/// 距离下限（Q14，≈0.01），避免除零/中心过亮溢出。
const MIN_D: i64 = 164;
/// π/2（Q14 弧度）。
const PI_OVER_2: i64 = 25736;
/// 起始角 11.0 弧度（Q14）。
const THETA0: i64 = 180224;
/// 每点角步进 π/30 弧度（Q14）。
const THETA_STEP: i64 = 1715;
/// 颜色幅度 0.1 × Q。
const COL_AMP: i64 = 1638;
/// 颜色幅度 0.09 × Q。
const COL_B_AMP: i64 = 1475;
/// 0.08 × Q（time → t 的弧度系数）。
const T_SCALE: i64 = 1311;

/// 256 项 cos 表：cos(2πk/256) * SCALE。
static mut COS: [i16; 256] = [0; 256];

fn init_cos(c: &mut [i16; 256]) {
    // Chebyshev 递推：cos(kθ) = 2·cos(θ)·cos((k-1)θ) − cos((k-2)θ)。
    const CD: i64 = 16379;
    let mut c0: i64 = 16384;
    let mut c1: i64 = CD;
    c[0] = (c0 * SCALE / 16384) as i16;
    c[1] = (c1 * SCALE / 16384) as i16;
    for k in 2..256 {
        let ck = (2 * CD * c1 / 16384) - c0;
        c0 = c1;
        c1 = ck;
        c[k] = (ck * SCALE / 16384) as i16;
    }
}

/// 查表 `cos(2πk/TURN) * SCALE`，k 任意整数（环绕 mod TURN）。
#[inline(always)]
fn cos_fixed(k: i32) -> i32 {
    let k = k & (TURN - 1);
    // SAFETY: COS 初始化于任何使用之前。
    unsafe { COS[k as usize] as i32 }
}

/// cos(a)，a 为 Q14 弧度，返回 cos * SCALE。
#[inline(always)]
fn cos_rad(a: i64) -> i64 {
    // 先归一化到 [0, 2π)（Q14），防止大角度乘 667543 溢出 i64。
    let tau = 2 * PI_OVER_2; // 2π（Q14）
    let a = a % tau;
    let a = if a < 0 { a + tau } else { a };
    // idx = a / (2π) * TURN，取模 TURN。256/(2π)≈40.7437，Q14 后 667543。
    let mut idx = (a * 667543) >> 28;
    idx %= TURN as i64;
    if idx < 0 {
        idx += TURN as i64;
    }
    cos_fixed(idx as i32) as i64
}

/// sin(a)，a 为 Q14 弧度，返回 sin * SCALE。
#[inline(always)]
fn sin_rad(a: i64) -> i64 {
    cos_rad(a - PI_OVER_2)
}

/// 整数平方根（向下取整）。
#[inline(always)]
fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut x = v;
    let mut y = (v + 1) / 2;
    while y < x {
        x = y;
        y = (x + v / x) / 2;
    }
    x
}

#[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();

    // SAFETY: 单线程启动，任何使用之前初始化。
    unsafe {
        init_cos(&mut *core::ptr::addr_of_mut!(COS));
    }

    let fb = fb_base() as *mut u32;
    let _ = writeln!(uart, "shader: 60-point orbital glow shader (0RGB)");

    let t0 = read_mtime();

    // 每点固定数据（与 t 无关）：size 按原始全局索引 g = i*G/N 计算，
    // 这样降低 N 只会让点变稀疏，而大小/覆盖范围保持不变。
    let mut size_q = [0i64; N];
    for (i, s) in size_q.iter_mut().enumerate() {
        let g = (i * G / N) as i64;
        *s = g * 82; // 0.005 * 16384 ≈ 82
    }

    loop {
        // t = time_s * 0.08（Q14 弧度），归一化到 [0, 2π) 防乘法溢出。
        let elapsed = read_mtime().wrapping_sub(t0);
        let tau = 2 * PI_OVER_2;
        let t_q = (elapsed as i64 * T_SCALE / MTIME_TICK_HZ as i64) % tau;

        // ── 自适应窗口大小。──
        let disp = read_disp_size();
        let disp_w = disp.width.clamp(2, FB_WIDTH);
        let disp_h = disp.height.clamp(2, FB_WIDTH);
        let iw = (disp_w / UP).max(1);
        let ih = (disp_h / UP).max(1);
        let m = iw.min(ih) as i64; // 纵横比基准

        // ── 预计算 N 个点的位置与颜色乘积（Q14 / Q28）。──
        // 全局索引 g = i*G/N：采样原始 60 点的子集，保持分布。
        let mut px = [0i64; N];
        let mut py = [0i64; N];
        let mut pr = [0i64; N];
        let mut pg = [0i64; N];
        let mut pb = [0i64; N];
        for i in 0..N {
            let g = (i * G / N) as i64;
            // 每点角步进仍按原始 60 点覆盖整圈（THETA_STEP * g）。
            let theta_g = THETA0 + THETA_STEP * g;
            // pos = (cos(θ·t)·r, sin(θ−t)·r)
            px[i] = cos_rad(theta_g * t_q / Q) * R / SCALE;
            py[i] = sin_rad(theta_g - t_q) * R / SCALE;
            // 颜色：r=0.1·cos(t·g), g=0.1·sin(t·g), b=0.09·sin(g)
            let col_r = cos_rad(t_q * g) * COL_AMP / SCALE;
            let col_g = sin_rad(t_q * g) * COL_AMP / SCALE;
            let col_b = sin_rad(g * Q) * COL_B_AMP / SCALE;
            // product = color × size（Q28）
            pr[i] = col_r * size_q[i];
            pg[i] = col_g * size_q[i];
            pb[i] = col_b * size_q[i];
        }

        // ── 渲染。──
        for ry in 0..ih {
            for rx in 0..iw {
                // surfacePosition：归一化到 [-1,1]，保持纵横比。
                let sx = (2 * rx as i64 - iw as i64) * Q / m;
                let sy = (2 * ry as i64 - ih as i64) * Q / m;
                let mut acc_r = 0i64;
                let mut acc_g = 0i64;
                let mut acc_b = 0i64;
                for i in 0..N {
                    let dx = px[i] - sx;
                    let dy = py[i] - sy;
                    let d = isqrt(dx * dx + dy * dy).max(MIN_D);
                    acc_r += pr[i] / d;
                    acc_g += pg[i] / d;
                    acc_b += pb[i] / d;
                }
                let vr = (acc_r * 255 / Q).clamp(0, 255) as u32;
                let vg = (acc_g * 255 / Q).clamp(0, 255) as u32;
                let vb = (acc_b * 255 / Q).clamp(0, 255) as u32;
                let v = vb | (vg << 8) | (vr << 16);
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

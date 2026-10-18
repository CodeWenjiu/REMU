//! `f32` primitives behind the solver, dispatched to the selected backend
//! (`math-std` / `math-libm` / `math-micromath`). The micromath backend keeps
//! its ~0.002 trig accuracy but refines `sqrt` (micromath's own is a ~5% bit
//! trick).

#[cfg(feature = "math-std")]
pub(crate) fn sin(x: f32) -> f32 {
    x.sin()
}

#[cfg(feature = "math-libm")]
pub(crate) fn sin(x: f32) -> f32 {
    libm::sinf(x)
}

#[cfg(feature = "math-micromath")]
pub(crate) fn sin(x: f32) -> f32 {
    micromath::F32Ext::sin(x)
}

#[cfg(feature = "math-std")]
pub(crate) fn cos(x: f32) -> f32 {
    x.cos()
}

#[cfg(feature = "math-libm")]
pub(crate) fn cos(x: f32) -> f32 {
    libm::cosf(x)
}

#[cfg(feature = "math-micromath")]
pub(crate) fn cos(x: f32) -> f32 {
    micromath::F32Ext::cos(x)
}

#[cfg(feature = "math-std")]
pub(crate) fn atan2(y: f32, x: f32) -> f32 {
    y.atan2(x)
}

#[cfg(feature = "math-libm")]
pub(crate) fn atan2(y: f32, x: f32) -> f32 {
    libm::atan2f(y, x)
}

#[cfg(feature = "math-micromath")]
pub(crate) fn atan2(y: f32, x: f32) -> f32 {
    micromath::F32Ext::atan2(y, x)
}

#[cfg(feature = "math-std")]
pub(crate) fn sqrt(x: f32) -> f32 {
    x.sqrt()
}

#[cfg(feature = "math-libm")]
pub(crate) fn sqrt(x: f32) -> f32 {
    libm::sqrtf(x)
}

#[cfg(feature = "math-micromath")]
pub(crate) fn sqrt(x: f32) -> f32 {
    // Newton-refine micromath's ~5% seed on the inverse (multiplication-only);
    // two steps leave a relative error of a few 1e-6.
    let r = micromath::F32Ext::invsqrt(x);
    let r = r * (1.5 - 0.5 * x * r * r);
    let r = r * (1.5 - 0.5 * x * r * r);
    x * r
}

#[cfg(test)]
mod tests {
    use core::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, SQRT_2};

    use super::*;

    #[test]
    fn trig_and_roots_sane() {
        // Trig tolerances cover micromath's documented 0.002 max error; the
        // refined sqrt holds f32-grade accuracy on every backend.
        assert!(sin(0.0).abs() < 5e-3);
        assert!((sin(FRAC_PI_2) - 1.0).abs() < 5e-3, "{}", sin(FRAC_PI_2));
        assert!((cos(0.0) - 1.0).abs() < 5e-3);
        assert!((cos(PI) + 1.0).abs() < 5e-3, "{}", cos(PI));
        assert!((atan2(1.0, 1.0) - FRAC_PI_4).abs() < 5e-3);
        assert!((sqrt(4.0) - 2.0).abs() < 1e-4);
        assert!((sqrt(2.0) - SQRT_2).abs() < 1e-4, "{}", sqrt(2.0));
    }
}

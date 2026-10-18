//! `Vec3` and `Quat` newtypes over `[f32; N]` with operator overloads.

use core::ops::{Add, Index, IndexMut, Mul, Sub};

use crate::math;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3(pub [f32; 3]);

/// Rotation quaternion, `(w, x, y, z)`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat(pub [f32; 4]);

impl Vec3 {
    pub const ZERO: Self = Self([0.0; 3]);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self([x, y, z])
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.0[0] * rhs.0[0] + self.0[1] * rhs.0[1] + self.0[2] * rhs.0[2]
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self([
            self.0[1] * rhs.0[2] - self.0[2] * rhs.0[1],
            self.0[2] * rhs.0[0] - self.0[0] * rhs.0[2],
            self.0[0] * rhs.0[1] - self.0[1] * rhs.0[0],
        ])
    }
}

impl Quat {
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 0.0]);

    pub const fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self([w, x, y, z])
    }

    /// Rotation of `angle` radians about the unit `axis`.
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let h = 0.5 * angle;
        let s = math::sin(h);
        Self([math::cos(h), axis[0] * s, axis[1] * s, axis[2] * s])
    }

    /// Rotates `v` by `self`.
    pub fn rotate(self, v: Vec3) -> Vec3 {
        let u = Vec3([self.0[1], self.0[2], self.0[3]]);
        let t = u.cross(v) * 2.0;
        v + t * self.0[0] + u.cross(t)
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
        ])
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self([
            self.0[0] - rhs.0[0],
            self.0[1] - rhs.0[1],
            self.0[2] - rhs.0[2],
        ])
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self([self.0[0] * rhs, self.0[1] * rhs, self.0[2] * rhs])
    }
}

/// Hamilton product `self ∘ rhs`.
impl Mul for Quat {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let (a, b) = (self.0, rhs.0);
        Self([
            a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
            a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
            a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
            a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
        ])
    }
}

impl Index<usize> for Vec3 {
    type Output = f32;

    fn index(&self, index: usize) -> &f32 {
        &self.0[index]
    }
}

impl IndexMut<usize> for Vec3 {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.0[index]
    }
}

impl Index<usize> for Quat {
    type Output = f32;

    fn index(&self, index: usize) -> &f32 {
        &self.0[index]
    }
}

impl IndexMut<usize> for Quat {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.0[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Backend-agnostic tolerance: micromath trig caps accuracy at ~0.002.
    const TOL: f32 = 5.0e-3;

    #[test]
    fn operators_act_elementwise() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vec3::new(3.0, 3.0, 3.0));
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert!((a.dot(b) - 32.0).abs() < 1.0e-6);
        assert_eq!(
            Vec3::new(1.0, 0.0, 0.0).cross(Vec3::new(0.0, 1.0, 0.0)),
            Vec3::new(0.0, 0.0, 1.0)
        );
    }

    #[test]
    fn quarter_turn_maps_x_to_y() {
        let q = Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), core::f32::consts::FRAC_PI_2);
        let v = q.rotate(Vec3::new(1.0, 0.0, 0.0));
        assert!(v[0].abs() < TOL && (v[1] - 1.0).abs() < TOL, "{v:?}");
    }
}

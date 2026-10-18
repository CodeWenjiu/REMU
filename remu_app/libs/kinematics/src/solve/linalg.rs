//! Small dense linear algebra behind the solver: Cholesky factorization and
//! solution of the `JᵀJ + λ²I` normal equations.

use crate::math;

pub(crate) fn dot6(a: &[f32; 6], b: &[f32; 6]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Cholesky factorization of an SPD matrix; `None` when a pivot is not
/// finite and positive (including non-finite inputs).
pub(crate) fn cholesky<const NJ: usize>(mut a: [[f32; NJ]; NJ]) -> Option<[[f32; NJ]; NJ]> {
    for i in 0..NJ {
        for j in 0..=i {
            let mut s = a[i][j];
            for k in 0..j {
                s -= a[i][k] * a[j][k];
            }
            if i == j {
                if !s.is_finite() || s <= 1.0e-9 {
                    return None;
                }
                a[i][j] = math::sqrt(s);
            } else {
                a[i][j] = s / a[j][j];
            }
        }
    }
    Some(a)
}

/// Solve `L Lᵀ x = b` with the lower factor from [`cholesky`].
pub(crate) fn chol_solve<const NJ: usize>(l: &[[f32; NJ]; NJ], mut b: [f32; NJ]) -> [f32; NJ] {
    for i in 0..NJ {
        let mut s = b[i];
        for k in 0..i {
            s -= l[i][k] * b[k];
        }
        b[i] = s / l[i][i];
    }
    for i in (0..NJ).rev() {
        let mut s = b[i];
        for k in i + 1..NJ {
            s -= l[k][i] * b[k];
        }
        b[i] = s / l[i][i];
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cholesky_solves_a_known_system() {
        // [[4, 1], [1, 3]] x = [1, 2] → x = [1/11, 7/11].
        let l = cholesky([[4.0, 1.0], [1.0, 3.0]]).unwrap();
        let x = chol_solve(&l, [1.0, 2.0]);
        // Backend-agnostic bound: micromath's refined sqrt holds ~1e-6
        // relative accuracy.
        assert!((x[0] - 1.0 / 11.0).abs() < 1.0e-4, "{x:?}");
        assert!((x[1] - 7.0 / 11.0).abs() < 1.0e-4, "{x:?}");
    }

    #[test]
    fn cholesky_rejects_non_positive_pivots() {
        assert!(cholesky([[1.0, 2.0], [2.0, 1.0]]).is_none());
        assert!(cholesky([[f32::NAN, 0.0], [0.0, 1.0]]).is_none());
    }
}

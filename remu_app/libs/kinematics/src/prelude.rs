//! Public API surface of `kinematics`. Import via `use kinematics::prelude::*;`.

pub use crate::chain::{Chain, Joint};
pub use crate::solver::{IkError, Residual, Solution, Task};
pub use crate::vec::{Quat, Vec3};

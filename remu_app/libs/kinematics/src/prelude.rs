//! Public API surface of `kinematics`. Import via `use kinematics::prelude::*;`.

pub use crate::chain::{Chain, Joint, Links};
pub use crate::solve::{Avoiding, Basic, Capsule, IkError, Residual, Solution, SolvePolicy, Task};
pub use crate::vec::{Quat, Vec3};

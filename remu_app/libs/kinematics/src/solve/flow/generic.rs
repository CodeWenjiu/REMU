//! Compile-time solver capability configuration.

/// Compile-time solver capability selection; implement it to pick any subset
/// of the layers for a custom solver instantiation.
pub trait SolvePolicy {
    /// Fold capsule-obstacle clearance rows into the normal equations.
    const OBSTACLES: bool;
}

/// Task rows (position, orientation, posture) and joint limits only; the
/// base layer behind `Chain::reach`.
pub struct Basic;

/// [`Basic`] plus the obstacle-avoidance layer (`Task::obstacles`).
pub struct Avoiding;

impl SolvePolicy for Basic {
    const OBSTACLES: bool = false;
}

impl SolvePolicy for Avoiding {
    const OBSTACLES: bool = true;
}

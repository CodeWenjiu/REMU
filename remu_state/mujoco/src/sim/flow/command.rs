//! Obstacle command published by the arm view and applied by the core.

/// Obstacle command channel: the armview UI publishes mocap body positions and
/// the core applies the latest set before stepping. A triple buffer keeps the
/// UI non-blocking and always delivers the newest drag position.
#[derive(Clone, Default)]
pub struct MocapCommand {
    /// Flat `x y z` per mocap body, in mocap-index order.
    pub pos: Vec<f64>,
}

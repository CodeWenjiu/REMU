//! Latest arm state published by the core to the monitor.

/// Latest arm state, published by the core and consumed by the armview render
/// thread (values cross threads through a lock-free triple buffer; the mirror
/// session re-derives everything else).
#[derive(Default)]
pub struct ArmSnapshot {
    pub qpos: Vec<f64>,
    pub qvel: Vec<f64>,
    /// Mocap body positions (flat `x y z`), so the monitor sees obstacles
    /// exactly where the physics does.
    pub mocap: Vec<f64>,
    /// Model-defined home pose per mocap body (flat `x y z`); restored by the
    /// arm view's "place" action.
    pub mocap_home: Vec<f64>,
}

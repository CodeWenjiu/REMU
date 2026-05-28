//! [`IsaKind`] — contract for classifying [`IsaSpec`] per simulation backend.

use super::IsaSpec;

/// Each simulation backend exposes its own enum (e.g. `RemuIsaKind`, `NzeaIsaKind`) and classifies
/// [`IsaSpec`]. Unsupported combinations must panic with a clear message.
pub trait IsaKind: Copy + Eq + std::fmt::Debug {
    /// Panics if `spec` is not supported on this backend.
    fn from_isa_spec_or_panic(spec: &IsaSpec) -> Self;
}

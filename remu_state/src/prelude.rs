//! Public API surface of `remu_state`. Import via `use remu_state::prelude::*;`.

pub use crate::StateCmd;
pub use crate::bus::ObserverEvent;
pub use crate::error::StateError;
pub use crate::{StateFastProfile, StateMmioProfile, StatePolicy};

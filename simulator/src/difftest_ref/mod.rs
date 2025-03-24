use std::path::PathBuf;

use remu_utils::Simulators;

pub enum DifftestRefType {
    FFI {name: PathBuf},
    BuildIn {name: Simulators},
}

pub trait DifftestRefBuildIn {
    fn test_reg(&self) -> bool;
}

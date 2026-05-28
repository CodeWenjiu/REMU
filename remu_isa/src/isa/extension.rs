use crate::isa::reg::FprAccess;

pub trait Extension {
    const ENABLED: bool;
    type State: Default + Copy + PartialEq + std::fmt::Debug + FprAccess;
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Enabled<T>(pub T)
where
    T: Default + Copy + PartialEq + std::fmt::Debug + FprAccess;

impl<T> Extension for Enabled<T>
where
    T: Default + Copy + PartialEq + std::fmt::Debug + FprAccess,
{
    const ENABLED: bool = true;
    type State = T;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Disabled;

impl Extension for Disabled {
    const ENABLED: bool = false;
    type State = ();
}

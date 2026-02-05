#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RunState {
    #[default]
    Idle,
    Exit,
}

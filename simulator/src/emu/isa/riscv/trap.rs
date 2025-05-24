#[derive(PartialEq, Clone, Copy, Default)]
pub enum Trap {
    #[default]
    Ebreak = 3,

    EcallM = 11,
}

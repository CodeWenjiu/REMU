#[derive(PartialEq, Clone, Copy)]
pub enum Trap {
    Ebreak = 3,

    EcallM = 11,
}

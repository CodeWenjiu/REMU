#[derive(clap::Args, Debug, Clone)]
pub struct RegOption {
    #[arg(short, long, default_value_t = 0x8000_0000)]
    pub init_pc: u32,
}

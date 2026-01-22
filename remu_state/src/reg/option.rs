#[derive(clap::Args, Debug, Clone)]
pub struct RegOption {
    #[arg(long, value_parser = remu_fmt::parse_prefixed_uint::<u32>, default_value = "0x8000_0000")]
    pub init_pc: u32,
}

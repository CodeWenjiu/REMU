use crate::bus::MemRegionSpec;

#[derive(clap::Args, Debug, Clone)]
pub struct BusOption {
    #[arg(
        long = "mem",
        value_name = "NAME@START:END",
        action = clap::ArgAction::Append,
        default_value = "ram@0x8000_0000:0x8800_0000"
    )]
    pub mem: Vec<MemRegionSpec>,
}

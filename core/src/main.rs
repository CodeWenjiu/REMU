use option_parser::config_parser;

type Result = option_parser::Result<()>;

#[snafu::report]
fn main() -> Result {
    config_parser()
}

use option_parser::parser;

type Result = option_parser::Result<()>;

#[snafu::report]
fn main() -> Result {
    parser()?;

    Ok(())
}

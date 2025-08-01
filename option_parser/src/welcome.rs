use cfonts::{say, Colors, Fonts, Options};
use logger::Logger;
use owo_colors::OwoColorize;

use remu_utils::Platform;

const REMU: &str = r#"
,------. ,------.,--.   ,--.,--. ,--. 
|  .--. '|  .---'|   `.'   ||  | |  | 
|  '--'.'|  `--, |  |'.'|  ||  | |  | 
|  |\  \ |  `---.|  |   |  |'  '-'  ' 
`--' '--'`------'`--'   `--' `-----'  
    An Computer System Emulator!"#;

pub fn welcome(platform: &Platform) {
    Logger::show("Welcome to", Logger::CONGRATULATIONS);
    
    println!("{}", REMU.fg_rgb::<0x2E, 0x31, 0x92>().bold());

    let mut option = Options::default();
    option.font = Fonts::FontSimple;
    option.colors = vec![Colors::Red];
    option.spaceless = true;

    say(Options {
        text: platform.to_string(),
        ..option
    });

    println!();
}

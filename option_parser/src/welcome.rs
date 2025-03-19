use logger::Logger;
use owo_colors::OwoColorize;

use text_to_ascii_art::to_art;

const REMU: &str = r#"
                                                                            
                        ,------. ,------.,--.   ,--.,--. ,--. 
                        |  .--. '|  .---'|   `.'   ||  | |  | 
                        |  '--'.'|  `--, |  |'.'|  ||  | |  | 
                        |  |\  \ |  `---.|  |   |  |'  '-'  ' 
                        `--' '--'`------'`--'   `--' `-----'  
                                                                            
"#;

pub fn welcome(platform: &str) {
    Logger::show("Welcome to REMU - An Computer System Emulator!", Logger::CONGRATULATIONS);
    
    println!("{}", REMU.fg_rgb::<0x2E, 0x31, 0x92>().bold());

    println!("{}", to_art(platform.to_string(), "Standard", 0, 2, 0).unwrap());
}

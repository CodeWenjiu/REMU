use remu_macro::{log_error, log_todo};
use logger::Logger;

use super::{BaseApi, Mask};

pub enum Device {
    Serial,
}

impl From<&str> for Device {
    fn from(s: &str) -> Self {
        match s {
            "serial" => Device::Serial,
            _ => panic!("Invalid device type"),
        }
    }
}

pub struct Serial {

}

impl BaseApi for Serial {
    fn read(&mut self, _addr: u32, _mask: Mask) -> u32 {
        log_todo!();
        0
    }

    fn write(&mut self, _addr: u32, data: u32, mask: Mask) {
        match mask {
            Mask::Byte => {
                let c = data as u8 as char;
                print!("{}", c);
            }
            _ => {
                log_error!("Serial write only supports byte access");
            }
        }
    }
}

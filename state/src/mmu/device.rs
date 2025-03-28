use remu_macro::{log_error, log_todo};
use logger::Logger;

use super::Mask;

use enum_dispatch::enum_dispatch;

#[enum_dispatch]
#[derive(Debug)]
pub enum Device {
    Serial(Serial),
}

#[enum_dispatch(Device)]
pub trait DeviceIo {
    fn read(&mut self, addr: u32, mask: Mask) -> u32;
    fn write(&mut self, addr: u32, data: u32, mask: Mask);
}

impl Device {
    pub fn new(name: &str) -> Self {
        match name {
            "Serial" => Device::Serial(Serial::new()),
            _ => panic!("Invalid device type"),
        }
    }
}

#[derive(Debug)]
pub struct Serial {

}

impl Serial {
    pub fn new() -> Self {
        Serial {}
    }
}

impl DeviceIo for Serial {
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

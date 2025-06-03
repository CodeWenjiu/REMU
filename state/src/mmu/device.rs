use std::time::Instant;
use std::io::{self, Write};

use remu_macro::{log_error, log_todo};
use logger::Logger;

use super::Mask;

use enum_dispatch::enum_dispatch;

#[enum_dispatch]
#[derive(Debug)]
pub enum Device {
    Serial(Serial),
    Timer(Timer),
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
            "Timer"  => Device::Timer(Timer::new()),
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
                io::stdout().flush().unwrap();
            }
            _ => {
                log_error!("Serial write only supports byte access");
            }
        }
    }
}

#[derive(Debug)]
pub struct Timer {
    pub start_time: Instant
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            start_time: Instant::now()
        }
    }
}

impl DeviceIo for Timer {
    fn read(&mut self, addr: u32, _mask: Mask) -> u32 {
        let elapsed_duration = self.start_time.elapsed(); 
        let elapsed_micros = elapsed_duration.as_micros();

        match addr {
            0 => (elapsed_micros & 0xFFFFFFFF) as u32,
            4 => ((elapsed_micros >> 32) & 0xFFFFFFFF) as u32,
            _ => panic!("Invalid timer read address"),
        }
    }

    fn write(&mut self, _addr: u32, _data: u32, _mask: Mask) {
        log_todo!();
    }
}

remu_macro::mod_flat!(error, platform);

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ITRACE")] {
        use std::{cell::RefCell, rc::Rc};
        use owo_colors::OwoColorize;
        remu_macro::mod_flat!(disassembler);
    }
}

#[derive(Debug, Clone)]
pub struct ItraceConfigtionalWrapper {
    #[cfg(feature = "ITRACE")]
    pub disassembler: Rc<RefCell<Disassembler>>,
}

impl ItraceConfigtionalWrapper {
    pub fn new(_isa: ISA) -> Self {
        Self {
            #[cfg(feature = "ITRACE")]
            disassembler: Rc::new(RefCell::new(Disassembler::new(_isa).unwrap())),
        }
    }

    pub fn try_analize(
        &self,
        data: u32,
        addr: u32,
    ) {
        print!(
            "{}",
            self.try_analize_fmt(data, addr)
        );
    }

    pub fn try_analize_fmt(
        &self,
        _data: u32,
        _addr: u32,
    ) -> String {
        #[cfg(feature = "ITRACE")]
        return self.disassembler
            .borrow()
            .try_analize(_data, _addr)
            .magenta()
            .to_string();

        // If ITRACE is not enabled, return an empty string
        #[cfg(not(feature = "ITRACE"))]
        return format!("0x{:08x}", _data).to_string();
    }
}

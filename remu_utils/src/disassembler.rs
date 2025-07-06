use remu_macro::log_err;

cfg_if::cfg_if! {
if #[cfg(feature = "ITRACE")] {
    use capstone::prelude::*;
    use crate::ISA;
    
    #[derive(Debug)]
    pub struct Disassembler {
        pub disasm: Capstone,
    }
    
    impl Disassembler {
    
        pub fn new(isa: ISA) -> Result<Self, ()> {
            let _ = isa;

            let disasm = log_err!(
                Capstone::new()
                    .riscv()
                    .mode(arch::riscv::ArchMode::RiscV32)
                    .detail(true)
                    .build(), 
                
                ()
            )?;

            Ok(Self { 
                disasm
            })
        }
    
        pub fn disasm(&self, code: &[u8], addr: u64) -> String {
            let inst = 
                self.disasm.disasm_count(code, addr, 1).unwrap();

            let inst = &inst[0];

            let mnemonic = inst.mnemonic().unwrap_or("<unknown>");
            let op_str = inst.op_str().unwrap_or("");

            format!(
                "{} {}",
                mnemonic,
                op_str
            )
        }
    
        pub fn disasm_suit(&self, code: u32, addr: u64) -> Option<String> {
            let code = code.to_le_bytes();

            let inst = 
                self.disasm.disasm_count(&code, addr, 1).unwrap();

            inst.get(0).map(|inst| {
                let mnemonic = inst.mnemonic();
                let op_str = inst.op_str();

                if let Some(mnemonic) = mnemonic {
                    if let Some(op_str) = op_str {
                        format!("{} {}", mnemonic, op_str)
                    } else {
                        mnemonic.to_string()
                    }
                } else {
                    "<unknown>".to_string()
                }
            })
        }
    
        pub fn try_analize(&self, code: u32, addr: u32) -> String {
            self.disasm_suit(code, addr.into()).map_or(
                // from ascii
                code.to_le_bytes().iter().map(|&x| { x as char }).collect(), 
                |f| f
            )
        }
    }
    

}
}


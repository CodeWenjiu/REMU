use std::ffi::CString;

cfg_if::cfg_if! {
if #[cfg(feature = "ITRACE")] {
    use llvm_sys::disassembler::*;
    use llvm_sys::target::*;
    
    use crate::ISA;
    
    #[derive(Debug, Clone, Copy)]
    pub struct Disassembler {
        pub disasm: LLVMDisasmContextRef,
    }
    
    impl Disassembler {
        fn isa2triple(isa: ISA) -> &'static str {
            match isa {
                ISA::RV32E => "riscv32-unknown-linux-gnu",
                ISA::RV32I => "riscv32-unknown-linux-gnu",
                ISA::RV32IM => "riscv32-unknown-linux-gnu",
            }
        }
    
        // https://llvm.org/docs/RISCVUsage.html#riscv-i2p1-note
        fn isa2feature(isa: ISA) -> &'static str {
            match isa {
                ISA::RV32E => "+e",
                ISA::RV32I => "+i",
                ISA::RV32IM => "+i,+m",
            }
        }
    
        pub fn new(isa: ISA) -> Result<Self, ()> {
            unsafe {
                let triple: CString = CString::new(Self::isa2triple(isa)).unwrap();
                let cpu: CString = CString::new("").unwrap();
                let feature: CString = CString::new(Self::isa2feature(isa)).unwrap();
    
                LLVM_InitializeAllAsmPrinters();
                LLVM_InitializeAllTargets();
                LLVM_InitializeAllAsmParsers();
                LLVM_InitializeAllTargetInfos();
                LLVM_InitializeAllTargetMCs();
                LLVM_InitializeAllDisassemblers();
    
                let disasm = LLVMCreateDisasmCPUFeatures(
                    triple.as_ptr() as *const i8,
                    cpu.as_ptr() as *const i8,
                    feature.as_ptr() as *const i8,
                    std::ptr::null_mut(),
                    0,
                    None as LLVMOpInfoCallback,
                    None,
                );
    
                if disasm.is_null() {
                    Err(())
                } else {
                    Ok(Self { disasm })
                }
            }
        }
    
        pub fn disasm(&self, code: &[u8], addr: u64) -> String {
            unsafe {
                let mut inst_str = [0u8; 50];
    
                LLVMDisasmInstruction(
                    self.disasm,
                    code.as_ptr() as *mut u8,
                    4,
                    addr,
                    inst_str.as_mut_ptr() as *mut i8,
                    50,
                );
    
                String::from_utf8_lossy(&inst_str).to_string()
            }
        }
    
        pub fn disasm_suit(&self, code: u32, addr: u64) -> Option<String> {
            let result = self
                .disasm(&code.to_le_bytes(), addr)
                .replace("\0", "")
                .trim()
                .split_ascii_whitespace()
                .map(|x| format!("{} ", x))
                .collect::<String>();
    
            if result == "unimp " || result == "" {
                None
            } else {
                Some(result)
            }
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


use std::ffi::CString;

use llvm_sys::disassembler::*;
use llvm_sys::target::*;

use crate::SimpleDebugger;

#[derive(Debug, Clone)]
pub struct Disassembler {
    pub disasm: LLVMDisasmContextRef,
}

impl Disassembler {
    pub fn new(triple: &str) -> Result<Self, ()> {
        unsafe {
            let triple: CString = CString::new(triple).unwrap();
            let cpu: CString = CString::new("").unwrap();
            let feature: CString = CString::new("+m,+a,+c,+f,+d").unwrap();

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
}

impl SimpleDebugger {
    pub fn disasm(&self, code: u32, addr: u64) -> String {
        self.disassembler
            .borrow()
            .disasm(&code.to_le_bytes(), addr)
            .replace("\0", "")
            .trim()
            .split_ascii_whitespace()
            .map(|x| format!("{} ", x))
            .collect::<String>()
    }
}

//! Default M-mode trap handlers.

use riscv::register::{mcause, mepc, mtval};
use riscv_rt::TrapFrame;

use crate::uart::Uart16550;
use core::fmt::Write;

#[inline(never)]
fn report_machine_trap(kind: &'static str) {
    let mc = mcause::read();
    let raw = mc.bits();
    let ep = mepc::read();
    let tv = mtval::read();
    let mut uart = Uart16550::default_base();
    let _ = write!(
        uart,
        "\r\n*** remu_hal: machine trap ({kind}) ***\r\n\
           mcause = 0x{raw:x} ({cause:?})\r\n\
           mepc   = 0x{ep:x}\r\n\
           mtval  = 0x{tv:x}\r\n",
        cause = mc.cause(),
    );
}

#[unsafe(no_mangle)]
extern "C" fn ExceptionHandler(_trap_frame: &TrapFrame) -> ! {
    report_machine_trap("exception");
    core::panic!("machine exception (details above on UART)");
}

#[unsafe(no_mangle)]
extern "C" fn DefaultHandler() -> ! {
    report_machine_trap("interrupt");
    core::panic!("machine interrupt (details above on UART)");
}

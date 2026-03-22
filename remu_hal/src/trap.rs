//! 默认 M 态陷阱处理：通过 UART 打印 `mcause` / `mepc` / `mtval` 后 [`panic!`]。
//!
//! 与 [riscv-rt] 约定一致：提供全局符号 [`ExceptionHandler`]（未单独实现的异常）和
//! [`DefaultHandler`]（未单独实现的中断）。应用只需正常依赖 `remu_hal` 即可链接进来。

use riscv::register::{mcause, mepc, mtval};
use riscv_rt::TrapFrame;

use crate::write_fmt;

#[inline(never)]
fn report_machine_trap(kind: &'static str) {
    let mc = mcause::read();
    let raw = mc.bits();
    let ep = mepc::read();
    let tv = mtval::read();
    write_fmt(format_args!(
        "\r\n*** remu_hal: machine trap ({kind}) ***\r\n\
           mcause = 0x{raw:x} ({cause:?})\r\n\
           mepc   = 0x{ep:x}\r\n\
           mtval  = 0x{tv:x}\r\n",
        cause = mc.cause(),
    ));
}

/// riscv-rt：标准异常表项最终都会落到此符号（若未单独实现某类异常）。
#[unsafe(no_mangle)]
extern "C" fn ExceptionHandler(_trap_frame: &TrapFrame) -> ! {
    report_machine_trap("exception");
    core::panic!("machine exception (details above on UART)");
}

/// riscv-rt：未单独实现的 M 态中断。
#[unsafe(no_mangle)]
extern "C" fn DefaultHandler() -> ! {
    report_machine_trap("interrupt");
    core::panic!("machine interrupt (details above on UART)");
}

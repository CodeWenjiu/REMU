# Roadmap

Difftest 与泛型分层设计已实现并稳定，约定见 decisions.md。

下一步计划是继续完善difftest功能，

- 更明确的错误定义
目前的difftest方法仅仅只是返回一个bool表示是否出错，我希望使用thiserror定义更加明确的错误，包括出错的寄存器组，寄存器名，ref是什么数值，dut是什么数值

其中，寄存器名称可以从remu_types/src/isa/reg/gpr.rs中通过strum定义的enum实现数字到字符串的转换

- 更完善的报错提示
目前的difftest从前往后，只要查到一个错误就直接返回，但实际上可能存在多个位置不匹配，我们应该先完整遍历，将所有不匹配的点返回

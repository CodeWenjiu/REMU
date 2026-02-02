# 项目介绍

本项目旨在提供一个性能取向，调试功能丰富的解释型虚拟机，目前支持riscv32

## Crates
- remu-cli: 最上层入口，负责将用户交互转成命令交给下层
- remu-debugger: 调试器，直辖 harness（其下为 simulator），负责定义、分发和执行各层命令
- remu-simulator: 虚拟机，负责执行指令，模拟硬件行为
- remu-state: 掌控所有状态量，包括寄存器文件、内存、设备等
- ...其他的基本是辅助用crate,不予赘述

文档中提到的所有文件地址均以项目 \<remu\> 根目录为基准的相对路径

关于设计哲学，涉及多模块交互，实现方法，见.cursor/decisions.md
关于工作流，涉及如何运行，调试，benchmark等，见.cursor/workflows.md

当前最高优先级的待办事项，详见.cursor/roadmap.md

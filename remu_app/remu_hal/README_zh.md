# remu_hal

[English](README.md) | **简体中文**

面向在 remu 上运行的 **RISC-V bare-metal（`no_std`）** 程序的 **嵌入式 HAL**：**`riscv-rt`**、**`embedded-hal` / `embedded-io`**、UART、堆、陷阱、退出。

**目标三元组、Zve、xtask：** [README-targets.md](README-targets.md)  
**仓库总览：** [README.md](../README.md) · [README_zh.md](../README_zh.md)

---

## 1. 设计目标

在 **`no_std`** 环境下，把 remu 模拟器暴露的 **MMIO 设备**（UART、测试结束寄存器、CLINT 等）封装成 **可复用的 HAL**，并与 **嵌入式 Rust 生态**对齐：

- 使用 **`riscv-rt`** 提供启动、链接脚本衔接（`memory.x` + `link.x`）及陷阱向量约定。
- 使用 **`embedded-hal`（1.x）** 作为通用嵌入式抽象边界，便于日后为 GPIO、延时等实现标准 trait，并与社区驱动兼容。
- 使用 **`embedded-io`** 为 **UART** 等设备提供 **`Read` / `Write`** 等字节流接口（当前 **16550 UART** 已实现 **`embedded_io::Write`**）。
- 使用 **`riscv`** 寄存器访问与 **`critical-section`**（`critical-section-single-hart`）满足单核环境下的临界区约定。
- 使用 **`embedded-alloc`（LlffHeap）** + 链接脚本中的堆符号，提供 **`Vec` / `String` / `Box`** 等 **`alloc`** 能力。
- 使用 **`panic-halt`** 作为默认 panic 策略；未处理 M 态陷阱时由 **`remu_hal`** 打印 **`mcause` / `mepc` / `mtval`** 后再 panic。

应用侧 **只需依赖 `remu_hal`** 即可获得 `#[entry]`、`println!`、UART、退出、堆初始化等，而无需在每个 app 里重复拉取一长串嵌入式依赖。

---

## 2. 模块概览

| 模块 / 能力 | 说明 |
|-------------|------|
| **`riscv-rt`** | 重导出 **`entry`**；`build.rs` 将 **`memory.x`** 加入链接搜索路径，与 **`riscv-rt`** 的 **`link.x`** 配合。 |
| **`cpu::pre_main_init`** | 入口前 CPU / 环境初始化（需在 `main` 最早处调用，或由 **`heap::init`** 间接调用）。 |
| **`uart::Uart16550`** | MMIO **16550** 风格发送；实现 **`embedded_io::Write`** 与 **`core::fmt::Write`**，默认基址见 **`addresses`**。 |
| **`print` / `println!`** | 通过默认 UART 输出格式化日志。 |
| **`time`** | 读取 CLINT **`mtime`**（remu 中 **10 MHz** 等约定见源码注释）。 |
| **`exit_success` / `exit_failure`** | 写 **SiFive test finisher** 风格寄存器，通知 remu 正常 / 异常结束。 |
| **`heap::init`** | 初始化 **`#[global_allocator]`**；**任何堆分配前**须 **`unsafe { init() }`**（会调用 **`pre_main_init`**）。 |
| **`trap`** | **`ExceptionHandler` / `DefaultHandler`**：未处理陷阱时 UART 报错文后 panic。 |

更多 **目标三元组、Zve 扩展、xtask 一键命令**，见 **[README-targets.md](README-targets.md)**。

---

## 3. 示例应用（`remu_app/*`）

以下均为 **`no_std` + `riscv-rt` 入口** 的 **独立 Cargo 包**，通过 **`remu_hal`** 链接到 remu 的机器模型；可在 **remu CLI** 下 **真实执行**（配合 workspace / xtask 的 **`build-std`** 与目标设置）。其中 **`mnist`** 是面向 **手写体数字（MNIST 风格）** 的检测 / 分类演示，而非泛化的“推理示例”。

| 包名 | 路径 | 作用 |
|------|------|------|
| **`remu_app_hello_world`** | `remu_app/hello_world` | 最小可运行程序：UART 打印 **`Hello World`** 与格式化输出，然后 **`exit_success`**。验证 **启动链、UART、退出语义**。 |
| **`remu_app_collection`** | `remu_app/collection` | **`heap::init`** 后使用 **`Vec` / `String` / `Box`** 等集合类型并打印结果。验证 **全局分配器、`alloc` 与 UART 输出**。 |
| **`remu_app_mnist`** | `remu_app/mnist` | **MNIST 手写数字识别**：内置量化全连接网络（784→256→128→10），对 **`test_images/`** 中的样本做推理并与标签比对；可选 **benchmark**（见 **`BENCHMARK_MODE`**）。用于验证 **堆、`alloc`、定点推理与较完整应用路径**。 |

构建与运行方式（含 **RV32IM**、**Zve32x** 等）请参考 **[README-targets.md](README-targets.md)** 中的 **`xtask print run-app`** / **`just run-app`** 或手动 **`cargo`** 命令；注意为应用构建使用 **`-Z build-std=core,alloc`**。

---

## 4. 依赖关系（概念图）

```
remu_app_*  ──depends on──►  remu_hal
                               │
       ┌───────────────────────┼───────────────────────┐
       ▼                       ▼                       ▼
  riscv-rt              embedded-io              embedded-hal
  riscv                 embedded-alloc           (trait 边界 / 扩展)
  critical-section      panic-halt
```

---

## 5. 小结

- **`remu_hal`** 在 **`riscv-rt` + `embedded-hal` / `embedded-io` + `riscv` + `embedded-alloc`** 等库之上，为 remu 的模拟硬件提供 **统一、可移植的嵌入式 HAL**。
- **`remu_app`** 下包含 **多个真实可运行的测试 / 示例程序**，从 **最小打印** 到 **堆与集合**、再到 **MNIST 手写数字识别**，便于回归模拟器与工具链行为。

若你扩展了新的 MMIO 设备，建议在同一风格下实现 **`embedded-hal` / `embedded-io`** 相应 trait，并在本文档中补充一节 **设备表**。

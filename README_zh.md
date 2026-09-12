# remu

[English](README.md) | **简体中文**

**remu** 是一套用 **Rust** 编写的 **RISC-V** 执行与调试框架。它将交互式前端与可插拔的 CPU 模拟器结合，支持与参考模型 **差分测试（difftest）**，并提供 **周期精确的 RTL 协同仿真** 后端（nzea，基于 Verilator）。

> **子模块说明：** 本仓库是更大项目 **[chip-dev](https://github.com/CodeWenjiu/chip-dev)** 中的 **git submodule**。受 **workspace / 路径依赖** 等限制，目前 **不能作为独立仓库完整跑通**。若要在本地 **构建与测试**，请 **整体拉取 [chip-dev](https://github.com/CodeWenjiu/chip-dev)（含 submodule）**，例如 `git clone --recursive https://github.com/CodeWenjiu/chip-dev.git`，并在该顶层工程下使用；**不要**只单独 clone `remu` 期望一键可用。

---

## 致谢

本项目在设计与思路上 **参考并受益于 [NEMU](https://github.com/NJU-ProjectN/nemu)**（南京大学教学用 ISA 模拟器）。单步执行、状态查看、上板与 bring-up 等与 NEMU 生态相近的流程，对 remu 的形态影响很大。**感谢 NEMU / Project-N 的作者与社区** 提供的优秀参考实现与文档。

---

## 性能

解释器核心针对 **稳态执行** 做了深度优化：译码分发是**单一扁平跳转表**；指令缓存让执行直接引用缓存行（省去栈物化）；PC 在批量运行中全程驻留寄存器；热路径访存走软件 TLB（addend 形式），命中路径不构造任何错误。

内置基准是 **`remu_app/microbench`**（移植自 [AM microbench](https://github.com/NJU-ProjectN/am-kernels) 套件），以参考 CPU 的分数为基准。在作者的笔记本（Core Ultra 5 125H）上：

| 模拟器   | microbench `ref` 分数 | guest 墙钟 |
|---------|----------------------:|-----------:|
| **remu** | **约 3800**            | 约 4.5 s   |
| Spike    | 约 3700（开启 `--real-time-clint`） | 约 7 s |

> 两个模拟器的原始 host cycles 相差约 10%。旧版 README 曾报告 Spike 快 2-3 倍——那是 **Spike 默认 mtime 按指令数推进** 造成的假象（guest 报时比墙钟快约 3.5 倍）。`just run-app … --platform spike` 现在会传 `--real-time-clint`，使 guest 时间跨平台可比。

分数会随 CPU 频率策略（注意 `scaling_governor`）、编译器与负载波动，仅作参考。若要做与频率无关的对比，请用 `perf stat -e cycles` 统计运行周期。

**复现方式**（本仓库内）：

```bash
just run-app microbench riscv32im --platform remu --app-args ref -- --platform remu --batch --startup continue
just run-app microbench riscv32im --platform spike --app-args ref
```

---

## 架构：前后端解耦与可插拔后端

remu 将 **调试器 / CLI（前端）与执行引擎（后端）分离**：

- **多种模拟器** 可作为后端接入：`remu`（内置 Rust ISA 模型）、`spike`（随仓库提供的 C++ 参考实现）、`nzea`（Verilator RTL 协同仿真）。
- 内置 **差分测试（difftest）**：DUT 与 **参考模型** 同步推进，对比寄存器与内存状态，尽早发现语义偏差（`--difftest remu` / `--difftest spike`）。
- **硬件 / RTL** 通过 `nzea` 参与：Verilator 生成的定制核心周期模型，经 DPI 与前端通信，复用同一套 difftest 与交互前端。
- `--platform none` 可在不带模拟器的情况下运行前端（例如仅调试命令）。

这样在更换或组合 **快速功能级模型**、**周期精确 RTL** 与 **黄金参考模型** 时，交互与调试流程可以保持稳定。

---

## 支持的 ISA

仅 **RV32**（默认 `--isa riscv32i`）：

| `--isa` 示例 | M | 向量（Zve32x，VLEN 128） | wjCus0（自定义） |
|-------------|---|-------------------------|-----------------|
| `riscv32i` / `rv32i` | | | |
| `riscv32im` / `rv32im` | ✓ | | |
| `rv32i_zve32x_zvl128b` | | ✓ | |
| `rv32im_zve32x_zvl128b` | ✓ | ✓ | |
| `riscv32i_wjCus0` / `riscv32im_wjCus0` | （im 版带 ✓） | | ✓ |

`wjCus0` 变体启用 MNIST 应用使用的自定义协处理器扩展。完整矩阵与 target 字符串处理见 [`remu_hal/README-targets.md`](remu_hal/README-targets.md)。

---

## 仓库结构概览

| 目录 | 作用 |
|------|------|
| `remu_cli` / `remu_debugger` | 交互式 shell 与调试命令 |
| `remu_simulator` | 模拟器抽象与具体后端：`simulators/remu`、`simulators/spike`、`simulators/nzea` |
| `remu_state`, `remu_types`, `remu_isa` | 体系结构状态、总线/设备、ISA 类型 |
| `remu_hal`, `remu_app/*` | 嵌入式 HAL（`riscv-rt`、`embedded-hal`、`embedded-io` 等）与可运行的 `no_std` 应用：`hello_world`、`collection`、`display`/`shader`、`microbench`、`mnist`、`nes`、`slint` — [English](remu_hal/README.md) · **[remu_hal/README_zh.md](remu_hal/README_zh.md)** |

`remu_state` 的总线模型包含内存区域与设备（UART 16550、SiFive test finisher、CLINT，以及基于 winit 窗口的交互式 `display` / `mouse` / `keyboard`）。设备与内存配置可通过 `--dev-base` / `--mem-base` 从文件加载。

---

## 环境与工作流（Nix、direnv、just）

**推荐开发环境由 Nix 提供**（[`flake.nix`](flake.nix)）：**Rust nightly**（含 `rust-src`、`clippy`、`rust-analyzer`、`llvm-tools-preview`）、**RISC-V bare-metal 目标**（`riscv32i` / `im` / `imac-unknown-none-elf`）、**Verilator**、**clang/libclang**、**mold**、**qemu**、**`just`** 等。

### Nix + direnv

1. 安装启用 **flakes** 的 [Nix](https://nixos.org/download.html)（在 `nix.conf` 中设置 `experimental-features = nix-command flakes`）。
2. 安装 [direnv](https://direnv.net/)，并在 **shell 里完成 hook**（bash/zsh/fish 等）。
3. 可选：[nix-direnv](https://github.com/nix-community/nix-direnv)，加快 dev shell 加载。
4. 克隆仓库并 **`cd` 到根目录**，在提示时执行一次 **`direnv allow`**。根目录 **`.envrc`** 使用 **`use flake`**，进入目录时会自动载入 flake 开发环境。

**不用 Nix 时：** 需自行准备兼容的 **Rust nightly**（本仓库 **Edition 2024**）、相同的 **RV32 bare-metal** 目标及主机工具；**以 flake 为准**。

### just

日常命令通过 **[just](https://github.com/casey/just)** 执行，配方写在根目录 [`justfile`](justfile)；**Nix shell 内已包含 `just`**。

| 命令 | 作用 |
|------|------|
| `just` | 列出所有配方 |
| `just build` | 调试构建：`cargo build -p remu_cli` |
| `just run -- 参数…` | **release** 运行 `remu_cli`，带 `RUST_BACKTRACE=1` |
| `just dev -- 参数…` | **debug** 运行 `remu_cli`，带 `RUST_BACKTRACE=1` |
| `just build-app APP [TARGET]` | 通过 **xtask** 构建嵌入式包 `remu_app_{APP}`（默认目标 `riscv32i`） |
| `just run-app APP [TARGET]` | 构建并在 remu 下运行该应用；`TARGET` 见 [`remu_hal/README-targets.md`](remu_hal/README-targets.md)（如 `riscv32im_zve32x_zvl128b`） |
| `just clean-app` | 删除 `target/app` 与 `target/app_zve32x` |
| `just clean-all` | `cargo clean` |

示例：

```bash
just run -- --help
just run-app hello_world
just run-app mnist riscv32im_zve32x_zvl128b
```

**`run-app` 平台选择：** `--platform` 配方参数会路由到不同的运行方式：

| `--platform` | 行为 |
|--------------|------|
| `remu`（默认） | 在 `remu_cli` 下用内置模拟器运行 |
| `spike` | 构建 Spike 适用的 ELF 并用原生 `spike` 二进制运行（带 `--real-time-clint`） |
| `qemu` | 用 `qemu-system-riscv32` 运行 |
| `host` | 在宿主机上原生运行应用（Rust `std`），不经过模拟器 |

**临时环境变量（`run-app` / 嵌入式 `cargo run`）：** 应用由 `remu_hal/scripts/remu-cargo-runner.sh` 拉起，其中通过 **xtask** 生成 `remu_cli` 命令。在与 **`just` 同一行**（或当前 shell）设置下列变量，即可传入对应 CLI 选项（需在 runner 执行时可见）：

| 变量 | 作用 |
|------|------|
| **`REMU_APP_ARGS`** | 通过 app-args 桥把参数传给嵌入式应用（`--app-args`）；例如 microbench 用 `REMU_APP_ARGS=ref` |
| **`DIFFTEST`** | 打开差分测试并指定参考模型：`spike` 或 `remu`；**不设 / 清空 = 关闭** |
| **`DEV`** | 若设置（任意值），`print run-remu` 用 **debug** 版宿主 `remu_cli`（`cargo run -p remu_cli` 不加 `--release`）。**嵌入式 `remu_app_*` 仍为 `--release`**（`run-app` / `build-app` 不变） |
| **`BATCH`** | 只要已设置（任意值），会加上 `--batch --startup continue`，用于非交互跑完 |

示例：在 **remu** 上以 `ref` 规模非交互跑 **microbench**：

```bash
REMU_APP_ARGS=ref BATCH=true just run-app microbench riscv32im
```

示例：以 **Spike** 为 difftest 参考跑 **mnist**：

```bash
DIFFTEST=spike just run-app mnist riscv32im_zve32x_zvl128b
```

另有 `look`、`step-sizes` 等用于汇编 / 体量分析，执行 **`just --list`** 查看。

### 直接使用 Cargo（在已载入的环境中）

```bash
cargo build --release -p remu_cli
cargo run -p remu_cli --release -- …
```

---

## 许可证

各组件许可证可能不同（例如随仓库提供的 Spike 自带 `LICENSE`）。请以各 crate 与第三方子树中的说明为准。

---

*本文档在 AI 辅助下编写，可能存在错漏；重要信息请以源码与实际测试为准。*

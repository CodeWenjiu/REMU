# remu

[English](README.md) | **简体中文**

**remu** 是一套用 **Rust** 编写的 **RISC-V** 执行与调试框架。它将交互式前端与可插拔的 CPU 模拟器结合，支持可选的与参考模型 **差分测试（difftest）**，并可通过适配接入 **RTL** 协同仿真（例如基于 **Verilator** 的周期模型）。

> **子模块说明：** 本仓库是更大项目 **[chip-dev](https://github.com/CodeWenjiu/chip-dev)** 中的 **git submodule**。受 **workspace / 路径依赖** 等限制，目前 **不能作为独立仓库完整跑通**。若要在本地 **构建与测试**，请 **整体拉取 [chip-dev](https://github.com/CodeWenjiu/chip-dev)（含 submodule）**，例如 `git clone --recursive https://github.com/CodeWenjiu/chip-dev.git`，并在该顶层工程下使用；**不要**只单独 clone `remu` 期望一键可用。

---

## 致谢

本项目在设计与思路上 **参考并受益于 [NEMU](https://github.com/NJU-ProjectN/nemu)**（南京大学教学用 ISA 模拟器）。单步执行、状态查看、上板与 bring-up 等与 NEMU 生态相近的流程，对 remu 的形态影响很大。**感谢 NEMU / Project-N 的作者与社区** 提供的优秀参考实现与文档。

---

## 性能

解释器核心针对 **稳态执行** 做了优化（译码分发、访存形态、热路径布局等）。在相同负载下，remu 的吞吐约可达作者先前 **用 C 编写的类 NEMU 实现的十倍**。

**负载：** [Abstract Machine (AM)](https://github.com/NJU-ProjectN/abstract-machine) **microbench**，**`ref` 规模**，ISA 为 **`riscv32im`**（RV32 + **M** 扩展）。下表各分数基于 **同一套二进制与主机环境**。

| 模拟器   | microbench-ref 分数 |
|---------|--------------------:|
| **remu** | **5503**            |
| Spike   | 11183               |
| QEMU    | 23468               |

*该基准分数越高越好；上表为单次参考测试，实际会随 CPU、编译器与编译选项变化。*

**参考运行环境**（上表分数在该机上测得；你的机器结果会不同）：

| | |
|--|--|
| **操作系统** | Linux **x86_64**，**WSL2**（内核 `6.6.87.2-microsoft-standard-WSL2`） |
| **CPU** | **Intel Core i5-13600KF**（虚拟机视角：**10 核 / 20 线程**，1 路） |
| **内存** | **约 32 GiB** |

**相对 remu 的速度（同一基准）：**

| 对比 **remu** | 相对速度 |
|--------------|----------|
| Spike        | ~2.0×    |
| QEMU         | ~4.3×    |
| 作者先前用 C 写的 NEMU | remu 约 **10×** 更快 |

**复现方式**（在完整拉取的 **[chip-dev](https://github.com/CodeWenjiu/chip-dev)** 仓库中，进入 **`am-zig/`** 目录执行）：

```bash
cd am-zig
BATCH=true just run <platform> riscv32 im am-microbench ref
```

将 **`<platform>`** 换成 `remu`、`spike`、`qemu` 等。**`BATCH=true`** 表示非交互跑完（与 remu 侧 `run-app` 里 `BATCH` 的用途一致）。

---

## 架构：前后端解耦与可插拔后端

remu 将 **调试器 / CLI（前端）与执行引擎（后端）分离**：

- 可将 **多种模拟器** 作为后端接入（例如内置 Rust ISA 模型、Spike，或自行扩展的适配层）。
- 内置 **差分测试（difftest）**：DUT 与 **参考模型** 同步推进，对比寄存器与内存状态，尽早发现语义偏差。
- **硬件 / RTL** 可通过合适适配参与——例如 **Verilator** 周期模型——从而在与软件模拟器相同的前端与 difftest 设施下调试 HDL。

这样在更换或组合 **快速功能级模型**、**周期精确 RTL** 与 **黄金参考模型** 时，交互与调试流程可以保持稳定。

---

## 支持的 ISA

目前仅 **RV32**（`--isa …`，默认 **`riscv32i`**）：

| `--isa` 示例 | M | 向量（Zve32x，VLEN 128） |
|-------------|---|-------------------------|
| `riscv32i` / `rv32i` | | |
| `riscv32im` / `rv32im` | ✓ | |
| `rv32i_zve32x_zvl128b` | | ✓ |
| `rv32im_zve32x_zvl128b` | ✓ | ✓ |

---

## 仓库结构概览

| 目录 | 作用 |
|------|------|
| `remu_cli` / `remu_debugger` | 交互式 shell 与调试命令 |
| `remu_simulator` | 模拟器抽象与具体后端（`remu`、Spike 等） |
| `remu_state`, `remu_types` | 体系结构状态、CSR、ISA 类型 |
| `remu_hal`, `remu_app/*` | 嵌入式 HAL（`riscv-rt`、`embedded-hal`、`embedded-io` 等）与可运行的 `no_std` 应用 — [English](remu_hal/README.md) · **[remu_hal/README_zh.md](remu_hal/README_zh.md)** |

---

## 环境与工作流（Nix、direnv、just）

**推荐开发环境由 Nix 提供**（[`flake.nix`](flake.nix)）：**Rust nightly**（含 `rust-src`、`clippy`、`rust-analyzer`、`llvm-tools-preview`）、**RISC-V bare-metal 目标**（`riscv32i` / `im` / `imac-unknown-none-elf`）、**Verilator**、**clang/libclang**、**mold**、**`just`** 等。

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

**临时环境变量（`run-app` / 嵌入式 `cargo run`）：** 应用由 `remu_hal/scripts/remu-cargo-runner.sh` 拉起，其中通过 **xtask** 生成 `remu_cli` 命令。在与 **`just` 同一行**（或当前 shell）设置下列变量，即可传入对应 CLI 选项（需在 runner 执行时可见）：

| 变量 | 作用 |
|------|------|
| **`PLATFORM`** | 传给 `remu_cli` 的 `--platform`：`remu`（CLI 默认）、`spike`、`nzea`、`none` |
| **`DIFFTEST`** | 打开差分测试并指定参考模型：`spike` 或 `remu`；**不设 / 清空 = 关闭** |
| **`DEV`** | 若设置（任意值），`print run-remu` 用 **debug** 版宿主 `remu_cli`（`cargo run -p remu_cli` 不加 `--release`）。**嵌入式 `remu_app_*` 仍为 `--release`**（`run-app` / `build-app` 不变） |
| **`BATCH`** | 只要已设置（任意值），会加上 `--batch --startup continue`，用于非交互跑完 |

示例：在 **remu** 平台上跑 **mnist**，并以 **Spike** 为 difftest 参考：

```bash
PLATFORM=remu DIFFTEST=spike just run-app mnist riscv32im_zve32x_zvl128b
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

# Spike Difftest 互操作 API 约定

## 设计

Spike 以**库模式**（`libspike.so`）运行：与 remu 同一进程，通过 FFI 调用。
`libspike.so` 由 Rust 侧在运行时按需构建（源文件 hash 检查 + 懒加载，见
`src/runtime.rs`），通过 `libloading` 加载符号表（见 `src/ffi.rs`）。

寄存器与内存在两侧各自持有：

```
┌─────────────────────────────────────────────────────────────┐
│  remu 进程                                                    │
│  ┌────────────────────┐      ┌────────────────────────────┐  │
│  │ DUT (remu)         │      │ Ref (spike via libspike)    │  │
│  │ State.reg (Rust)   │◄────►│ state_t (spike XPR/pc)      │  │
│  │ State.bus / Memory │─────►│ mem_t（spike 自有，初始复制）│  │
│  └────────────────────┘      └────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

- **内存**：spike 拥有自己的 `mem_t`（按 `difftest_mem_layout_t` 分配），
  初始镜像通过 `spike_difftest_copy_mem` 灌入；运行期间 DUT 的 MMIO 访问
  不传播给 spike（见下方“MMIO 步”）。
- **寄存器**：DUT 侧为 Rust `RiscvReg`，spike 侧为 `state_t`，通过
  `get_*_ptr` / `sync_regs_to_spike` 交换，无共享内存。

## ABI 布局（C 兼容）

### 寄存器块 `difftest_regs_t`

协议宽度**固定为 64 位**：C 函数签名编译期固定，无法按 XLEN 泛型化布局。
RV32 下值是零扩展存放，由 spike 侧按 `xlen` 分支转换回它的 `reg_t` 表示
（spike 内部 rv32 的 XPR/PC 为符号扩展）。

```c
typedef struct __attribute__((packed, aligned(8))) {
    uint64_t pc;
    uint64_t gpr[32];
} difftest_regs_t;   /* 264 bytes */
```

Rust 端对应（`src/ffi.rs`）：

```rust
#[repr(C)]
pub(crate) struct DifftestRegs {
    pub(crate) pc: u64,
    pub(crate) gpr: [u64; 32],
}
```

### 内存区域 `difftest_mem_layout_t`

只描述 guest 地址区间；宿主内存由 spike 自己分配。

```c
typedef struct {
    uintptr_t guest_base;
    size_t    size;
} difftest_mem_layout_t;
```

## C API（spike 侧实现，`src/difftest_abi.h`）

| 函数 | 作用 |
|------|------|
| `spike_difftest_init(layout, n_regions, init_pc, init_gpr, xlen, isa)` | 建 ctx：分配 mem_t、建 processor、按 XLEN 宽初值同步 pc/gpr |
| `spike_difftest_copy_mem(ctx, guest_base, data, len)` | 初始内存镜像复制进 spike 的 mem_t |
| `spike_difftest_read_mem / write_mem` | 内存回读/写入（debug 用） |
| `spike_difftest_step(ctx)` | 执行一条指令；返回 0 正常 / 1 程序退出 / -1 错误 |
| `spike_difftest_get_pc_ptr / get_gpr_ptr` | 返回 spike 内部状态指针（XLEN 宽，`const uint64_t*`） |
| `spike_difftest_get_csr(ctx, addr)` | 读 CSR，返回低 32 位（difftest 比较协议） |
| `spike_difftest_get_fpr(ctx, i)` | 读 FPR（仅 HAS_F 的 ISA） |
| `spike_difftest_sync_regs_to_spike(ctx, regs)` | 把 DUT 寄存器整块写回 spike（MMIO 步 / 调试写寄存器用） |
| `spike_difftest_get_vlenb / get_vr_ptr / sync_vr_to_spike / write_vr_reg` | V 扩展寄存器交换 |
| `spike_difftest_fini(ctx)` | 释放 ctx |

## Rust 侧职责（`src/simulator.rs`）

### 启动

- 用总线解析出的内存区域构造 `difftest_mem_layout_t` 数组；
- `init` 后逐段 `copy_mem` 灌入初始镜像；
- 断言 spike 报出的 `vlenb` 与配置一致（防 ISA 串味）。

### 每步（harness 驱动）

```
1. DUT step_once()                        // remu 执行一条
2. 若本步有 MMIO 访客访问：
     sync_regs_from(dut_reg)               // 寄存器整块同步给 spike，跳过 ref 执行
   否则：
     spike step                            // ref 执行同一条
     regs_diff(dut_reg)                    // 全寄存器比较
```

### 比较规则

- GPR/PC：两侧按 **XLEN mask** 比较（RV32 下 spike 为符号扩展表示）；
- CSR：按 `diff_mask()` 逐寄存器比较（Misa 等只读 CSR 由 ISA 常量提供）；
- FPR / V：仅在对应扩展存在时比较。

## 注意事项

- `difftest_regs_t` 是唯一的跨语言布局契约，改字段必须同步
  `src/ffi.rs`、`src/wrapper.cc` 与 `src/difftest_abi.h` 三处；
- `.so` 的新鲜度由源文件 hash（含 `wrapper.cc`、`difftest_abi.h`、spike
  源码树）保证，改动 C 侧后无需手动重建（首次运行自动 rebuild）。

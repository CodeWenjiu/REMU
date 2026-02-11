# Spike Difftest 互操作 API 约定

## 设计目标

1. **零拷贝**：remu 与 spike 访问同一段物理内存，无中间复制
2. **高性能**：单步 difftest 开销尽量小（寄存器布局紧凑、内存直接访问）

---

## 架构选择

**Spike 以库模式运行（libspike）**：与 remu 同一进程，通过 FFI 调用。这样寄存器与内存指针均可直接传递，天然零拷贝。

```
┌─────────────────────────────────────────────────────────┐
│  remu 进程                                                │
│  ┌──────────────┐    ┌─────────────────────────────────┐│
│  │ DUT (remu)   │    │ Ref (spike via libspike)          ││
│  │ State.reg    │◄──►│ 读/写同一块 regs 内存             ││
│  │ State.bus    │◄──►│ 读/写同一块 mem 内存              ││
│  └──────────────┘    └─────────────────────────────────┘│
│         │ 同一块内存（零拷贝）                            │
└─────────────────────────────────────────────────────────┘
```

---

## ABI 稳定布局（C 兼容）

### 1. 寄存器块 `difftest_regs_t`

remu 与 spike 共享的寄存器布局，需严格对齐以支持零拷贝。

```c
/* difftest_abi.h - ABI 稳定，remu 与 spike 共享 */

#define DIFFTEST_MAGIC   0x44534654  /* "DSFT" */
#define DIFFTEST_VERSION 1

/* 32 位 GPR，x0 恒为 0 由双方共同保证 */
typedef struct __attribute__((packed, aligned(8))) {
    uint32_t pc;
    uint32_t gpr[32];
    /* RV32IM 暂不包含 FPR，可扩展 */
} difftest_regs_t;

_Static_assert(sizeof(difftest_regs_t) == 132, "layout check");
_Static_assert(offsetof(difftest_regs_t, pc) == 0, "pc offset");
```

Rust 端对应：

```rust
#[repr(C, align(8))]
pub struct DifftestRegs {
    pub pc: u32,
    pub gpr: [u32; 32],
}
```

### 2. 内存区域 `difftest_mem_region_t`

描述一段共享内存：guest 地址区间 + 宿主指针。

```c
typedef struct {
    uintptr_t guest_base;  /* Guest 地址空间基址 (如 0x8000_0000) */
    void*     host_ptr;    /* 宿主可写指针，remu 与 spike 均直接访问 */
    size_t    size;
} difftest_mem_region_t;
```

- `host_ptr` 为 remu `Memory::storage` 的裸指针，spike 直接在此区间进行 load/store
- 支持多段（如 RAM + ROM），每段一个 `difftest_mem_region_t`

---

## C API（spike 侧实现）

### 初始化

```c
typedef struct spike_difftest_ctx spike_difftest_ctx_t;

/**
 * 初始化 spike difftest 上下文
 *
 * @param regs     寄存器块指针，双方直接读写，零拷贝
 * @param regions  内存区域数组，host_ptr 为共享存储
 * @param n_regions 区域数量
 * @param xlen     32 或 64
 * @param isa      如 "rv32im"
 * @return 上下文，失败返回 NULL
 */
spike_difftest_ctx_t* spike_difftest_init(
    difftest_regs_t*           regs,
    const difftest_mem_region_t* regions,
    size_t                     n_regions,
    uint32_t                   xlen,
    const char*                isa
);
```

### 单步执行

```c
/**
 * 执行一条指令
 *
 * 读 regs，从 regions 取指/访存，写回 regs
 * @return 0 正常，1 程序退出，-1 错误
 */
int spike_difftest_step(spike_difftest_ctx_t* ctx);
```

### 清理

```c
void spike_difftest_fini(spike_difftest_ctx_t* ctx);
```

---

## Rust 侧职责

### 1. 寄存器零拷贝

当 `--difftest spike` 时：

- 使用 `DifftestRegs` 作为 DUT 的寄存器存储（或让 `RiscvReg` 包装该块）
- 传给 `spike_difftest_init` 的 `regs` 即此 `DifftestRegs` 的 `*mut`，无额外复制

### 2. 内存零拷贝

- `Memory::storage` 使用连续 `Box<[u8]>` 或 `Vec<u8>`，取其 `as_mut_ptr()`
- 为每个 `Memory` 构造 `difftest_mem_region_t { guest_base: range.start, host_ptr: storage.as_mut_ptr(), size }`
- spike 在 `addr_to_mem` 中根据 `guest_base` 和 `host_ptr` 返回 `host_ptr + (addr - guest_base)`，实现零拷贝

### 3. 调用顺序（每步）

```
1. DUT (remu) step_once() → 更新 regs, mem
2. spike_difftest_step(ctx) → 读同一 regs/mem，执行，写回 regs
3. regs_diff(regs, dut_regs) → 比较（此时 regs 已是 spike 写回的结果）
```

---

## Spike 侧实现要点

### 1. 自定义 `abstract_mem_t`（或 `mem_t`）

实现一个后端，使用外部 `host_ptr` 而非内部 `sparse_memory_map`：

```cpp
class external_mem_t : public abstract_mem_t {
  uintptr_t guest_base_;
  char* host_base_;
  size_t size_;
public:
  external_mem_t(uintptr_t base, void* ptr, size_t sz);
  char* contents(reg_t addr) override {
    return host_base_ + (addr - guest_base_);
  }
  // load/store 直接操作 host_base_
};
```

### 2. 精简 processor 初始化

- 不启动 HTIF、设备树等，仅保留 processor + 外部内存
- 使用传入的 `difftest_regs_t*` 初始化 `state_t`，或每步同步到/从 `state_t`

### 3. 单步入口

- `spike_difftest_step` 内部：从 `regs` 加载 pc、XPR 到 processor，执行一条指令，写回 `regs`

---

## 性能建议

1. **寄存器**：`difftest_regs_t` 紧凑排列，避免 padding
2. **内存**：spike 的 `addr_to_mem` 使用 `host_base + (addr - guest_base)` 的 O(1) 计算，不做额外查找
3. **热路径**：`spike_difftest_step` 内避免 malloc、锁、系统调用
4. **可选**：对连续 RAM 做 last-hit cache，减少地址计算

---

## 扩展

- **FPR**：在 `difftest_regs_t` 中追加 `uint32_t fpr[32]` 或 `uint64_t fpr[32]`
- **CSR**：按需添加 `mstatus`, `mepc`, `mcause` 等字段
- **多 region**：`difftest_mem_region_t` 数组已支持，spike 按 `guest_base` 二分或线性查找

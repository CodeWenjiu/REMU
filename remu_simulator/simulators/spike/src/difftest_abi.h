#ifndef REMU_SPIKE_DIFFTEST_ABI_H
#define REMU_SPIKE_DIFFTEST_ABI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define DIFFTEST_MAGIC 0x44534654
#define DIFFTEST_VERSION 2

/** 32-bit GPR; x0 is always 0, enforced by both sides */
typedef struct __attribute__((packed, aligned(8))) {
    uint32_t pc;
    uint32_t gpr[32];
} difftest_regs_t;

/** Memory layout: base + size only; Spike owns the memory */
typedef struct {
    uintptr_t guest_base;
    size_t size;
} difftest_mem_layout_t;

/** Opaque context; owned by spike implementation */
typedef struct spike_difftest_ctx spike_difftest_ctx_t;

/**
 * Initialize spike difftest context.
 * Spike allocates mem_t itself; no external pointers held.
 *
 * @param layout    Memory layout array (guest_base, size)
 * @param n_regions Number of regions
 * @param init_pc   Initial PC
 * @param init_gpr  Initial GPR[32], may be NULL (then all zeros)
 * @param xlen      32 or 64
 * @param isa       e.g. "rv32im"
 * @return Context, or NULL on failure
 */
spike_difftest_ctx_t* spike_difftest_init(const difftest_mem_layout_t* layout,
                                          size_t n_regions,
                                          uint32_t init_pc,
                                          const uint32_t* init_gpr,
                                          uint32_t xlen,
                                          const char* isa);

/**
 * Copy initial memory content into Spike-owned memory
 */
void spike_difftest_copy_mem(spike_difftest_ctx_t* ctx,
                             uintptr_t guest_base,
                             const void* data,
                             size_t len);

/**
 * Sync DUT memory to Spike (for sync_from)
 */
void spike_difftest_sync_mem(spike_difftest_ctx_t* ctx,
                             uintptr_t guest_base,
                             const void* data,
                             size_t len);

/**
 * Read memory from Spike (for RefState / diff etc.)
 */
int spike_difftest_read_mem(spike_difftest_ctx_t* ctx,
                            uintptr_t addr,
                            void* buf,
                            size_t len);

/**
 * Write memory to Spike (for state bus write/set)
 */
int spike_difftest_write_mem(spike_difftest_ctx_t* ctx,
                             uintptr_t addr,
                             const void* data,
                             size_t len);

/**
 * Execute one instruction
 * @return 0 success, 1 program exit, -1 error
 */
int spike_difftest_step(spike_difftest_ctx_t* ctx);

/**
 * Get pointer to Spike's internal PC (reg_t).
 * For rv32, use low 32 bits. Valid until next step/sync.
 */
const uint32_t* spike_difftest_get_pc_ptr(spike_difftest_ctx_t* ctx);

/**
 * Get pointer to Spike's internal GPR[0].
 * Spike uses reg_t (uint64_t) per reg; for rv32, low 32 bits at offset 2*i.
 * I.e. (const uint32_t*)ptr, then gpr[i] = ptr[2*i]. Valid until next step/sync.
 */
const uint32_t* spike_difftest_get_gpr_ptr(spike_difftest_ctx_t* ctx);

/**
 * Read one CSR from Spike by address (e.g. 0x300 = mstatus).
 * Returns low 32 bits. For non-existent CSR, returns 0.
 */
uint32_t spike_difftest_get_csr(spike_difftest_ctx_t* ctx, uint16_t csr_addr);

/**
 * Sync regs to spike processor (for sync_from)
 */
void spike_difftest_sync_regs_to_spike(spike_difftest_ctx_t* ctx,
                                       const difftest_regs_t* regs);

/** Free context */
void spike_difftest_fini(spike_difftest_ctx_t* ctx);

#ifdef __cplusplus
}
#endif

#endif /* REMU_SPIKE_DIFFTEST_ABI_H */

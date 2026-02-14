/**
 * Spike difftest C interface wrapper.
 *
 * Spike owns its own memory and registers; no remu pointers are held.
 */

#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include <sys/syscall.h>
#include <unistd.h>

#include "difftest_abi.h"

#include "config.h"
#include "cfg.h"
#include "devices.h"
#include "encoding.h"
#include "processor.h"
#include "simif.h"
#include "trap.h"
#include "decode.h"
#include "decode_macros.h"
#include "mmu.h"
#include "vector_unit.h"

#include <cstdlib>
#include <cstring>
#include <map>
#include <sstream>
#include <vector>

/**
 * simif: holds mem_t, allocated by Spike
 */
class difftest_simif_t : public simif_t {
public:
    difftest_simif_t(std::vector<std::pair<reg_t, mem_t*>> mems, cfg_t* cfg)
        : mems_(std::move(mems)), cfg_(cfg) {}

    void set_proc(processor_t* proc) { harts_[0] = proc; }

    char* addr_to_mem(reg_t paddr) override {
        for (auto& [base, mem] : mems_) {
            reg_t size = mem->size();
            if (paddr >= base && paddr < base + size) {
                return mem->contents(paddr - base);
            }
        }
        return nullptr;
    }

    bool mmio_load(reg_t, size_t, uint8_t*) override { return false; }
    bool mmio_store(reg_t, size_t, const uint8_t*) override { return false; }
    void proc_reset(unsigned) override {}

    const cfg_t& get_cfg() const override { return *cfg_; }
    const std::map<size_t, processor_t*>& get_harts() const override {
        return harts_;
    }
    const char* get_symbol(uint64_t) override { return ""; }

    std::vector<std::pair<reg_t, mem_t*>>& mems() { return mems_; }

private:
    std::vector<std::pair<reg_t, mem_t*>> mems_;
    cfg_t* cfg_;
    std::map<size_t, processor_t*> harts_;
};

struct spike_difftest_ctx {
    std::string isa_str;
    cfg_t cfg;
    difftest_simif_t* simif;
    processor_t* proc;
};

static void sync_regs_to_spike(const difftest_regs_t* r, processor_t* p) {
    state_t* s = p->get_state();
    s->pc = r->pc;
    for (int i = 0; i < 32; i++) {
        s->XPR.write(i, r->gpr[i]);
    }
}

static mem_t* find_mem(spike_difftest_ctx_t* ctx, uintptr_t addr, reg_t* out_base) {
    for (auto& [base, mem] : ctx->simif->mems()) {
        if (addr >= base && addr < base + mem->size()) {
            *out_base = base;
            return mem;
        }
    }
    return nullptr;
}

extern "C" {

spike_difftest_ctx_t* spike_difftest_init(const difftest_mem_layout_t* layout,
                                          size_t n_regions,
                                          uint32_t init_pc,
                                          const uint32_t* init_gpr,
                                          uint32_t xlen,
                                          const char* isa)
{
    (void)xlen;  /* reserved for future rv64 support */
    if (!layout || n_regions == 0 || !isa) {
        return nullptr;
    }

    auto* ctx = new spike_difftest_ctx_t();
    ctx->isa_str = isa;
    ctx->cfg.isa = ctx->isa_str.c_str();
    ctx->cfg.priv = "m";
    ctx->cfg.hartids = {0};
    ctx->cfg.mem_layout.clear();
    ctx->cfg.pmpregions = 16;
    ctx->cfg.pmpgranularity = reg_t(1) << PMP_SHIFT;

    std::vector<std::pair<reg_t, mem_t*>> mems;
    for (size_t i = 0; i < n_regions; i++) {
        reg_t base = layout[i].guest_base;
        reg_t size = layout[i].size;
        ctx->cfg.mem_layout.emplace_back(base, size);
        mems.push_back({base, new mem_t(size)});
    }

    ctx->simif = new difftest_simif_t(std::move(mems), &ctx->cfg);
    static std::ostringstream null_out;
    ctx->proc = new processor_t(ctx->cfg.isa, ctx->cfg.priv, &ctx->cfg,
                                ctx->simif, 0, false, nullptr, null_out);
    ctx->simif->set_proc(ctx->proc);

    difftest_regs_t init_regs;
    init_regs.pc = init_pc;
    if (init_gpr) {
        for (int i = 0; i < 32; i++)
            init_regs.gpr[i] = init_gpr[i];
    } else {
        memset(init_regs.gpr, 0, sizeof(init_regs.gpr));
    }
    sync_regs_to_spike(&init_regs, ctx->proc);

    return ctx;
}

void spike_difftest_copy_mem(spike_difftest_ctx_t* ctx,
                             uintptr_t guest_base,
                             const void* data,
                             size_t len)
{
    if (!ctx || !ctx->simif || !data) return;

    reg_t base;
    mem_t* mem = find_mem(ctx, guest_base, &base);
    if (!mem) return;

    reg_t offset = guest_base - base;
    if (offset + len > mem->size()) return;

    /* mem->contents() returns single-page ptr only; use store() for page-by-page copy */
    mem->store(offset, len, const_cast<uint8_t*>(static_cast<const uint8_t*>(data)));
}

void spike_difftest_sync_mem(spike_difftest_ctx_t* ctx,
                             uintptr_t guest_base,
                             const void* data,
                             size_t len)
{
    spike_difftest_copy_mem(ctx, guest_base, data, len);
}

int spike_difftest_read_mem(spike_difftest_ctx_t* ctx,
                            uintptr_t addr,
                            void* buf,
                            size_t len)
{
    if (!ctx || !ctx->simif || !buf) return -1;

    reg_t base;
    mem_t* mem = find_mem(ctx, addr, &base);
    if (!mem) return -1;

    reg_t offset = addr - base;
    if (offset + len > mem->size()) return -1;

    /* mem->contents() returns single-page ptr only; use load() for page-by-page copy */
    if (!mem->load(offset, len, static_cast<uint8_t*>(buf)))
        return -1;
    return 0;
}

int spike_difftest_write_mem(spike_difftest_ctx_t* ctx,
                             uintptr_t addr,
                             const void* data,
                             size_t len)
{
    if (!ctx || !ctx->simif || !data) return -1;

    reg_t base;
    mem_t* mem = find_mem(ctx, addr, &base);
    if (!mem) return -1;

    reg_t offset = addr - base;
    if (offset + len > mem->size()) return -1;

    mem->store(offset, len, const_cast<uint8_t*>(static_cast<const uint8_t*>(data)));
    return 0;
}

int spike_difftest_step(spike_difftest_ctx_t* ctx)
{
    if (!ctx || !ctx->proc) return -1;

    /* Lazy regs: no sync after step. Rust reads directly from Spike state via get_*_ptr. */
    try {
        ctx->proc->step(1);
        return 0;
    } catch (trap_machine_ecall&) {
        /* Bare-metal: ecall with a0=93 (exit) or SYS_exit_group => program end.
         * TODO: For OS (Linux) support, let Spike handle ecall via htif/syscall proxy
         * instead of catching here. */
        reg_t a0 = ctx->proc->get_state()->XPR[17];
        if (a0 == 93 || a0 == SYS_exit_group) {
            return 1;
        }
        return -1;
    } catch (trap_t&) {
        return -1;
    }
}

const uint32_t* spike_difftest_get_pc_ptr(spike_difftest_ctx_t* ctx)
{
    if (!ctx || !ctx->proc) return nullptr;
    state_t* s = ctx->proc->get_state();
    return reinterpret_cast<const uint32_t*>(&s->pc);
}

const uint32_t* spike_difftest_get_gpr_ptr(spike_difftest_ctx_t* ctx)
{
    if (!ctx || !ctx->proc) return nullptr;
    state_t* s = ctx->proc->get_state();
    /* XPR is reg_t[32]; reg_t is uint64_t. For rv32, low 32 bits at 2*i. */
    return reinterpret_cast<const uint32_t*>(&s->XPR[0]);
}

uint32_t spike_difftest_get_csr(spike_difftest_ctx_t* ctx, uint16_t csr_addr)
{
    if (!ctx || !ctx->proc) return 0;
    reg_t val = ctx->proc->get_csr(static_cast<int>(csr_addr));
    return static_cast<uint32_t>(val);
}

uint32_t spike_difftest_get_fpr(spike_difftest_ctx_t* ctx, size_t index)
{
    if (!ctx || !ctx->proc || index >= 32) return 0;
    state_t* s = ctx->proc->get_state();
    return static_cast<uint32_t>(unboxF32(s->FPR[index]));
}

void spike_difftest_sync_regs_to_spike(spike_difftest_ctx_t* ctx,
                                       const difftest_regs_t* regs)
{
    if (ctx && ctx->proc && regs) {
        sync_regs_to_spike(regs, ctx->proc);
    }
}

size_t spike_difftest_get_vlenb(spike_difftest_ctx_t* ctx)
{
    if (!ctx || !ctx->proc || !ctx->proc->any_vector_extensions())
        return 0;
    return static_cast<size_t>(ctx->proc->VU.vlenb);
}

const uint8_t* spike_difftest_get_vr_ptr(spike_difftest_ctx_t* ctx)
{
    if (!ctx || !ctx->proc || !ctx->proc->any_vector_extensions() || ctx->proc->VU.vlenb == 0)
        return nullptr;
    return static_cast<const uint8_t*>(ctx->proc->VU.reg_file);
}

void spike_difftest_sync_vr_to_spike(spike_difftest_ctx_t* ctx,
                                     const uint8_t* data,
                                     size_t len)
{
    if (!ctx || !ctx->proc || !data)
        return;
    if (!ctx->proc->any_vector_extensions() || ctx->proc->VU.vlenb == 0)
        return;
    size_t vlenb = static_cast<size_t>(ctx->proc->VU.vlenb);
    if (len != 32 * vlenb)
        return;
    memcpy(ctx->proc->VU.reg_file, data, len);
}

void spike_difftest_write_vr_reg(spike_difftest_ctx_t* ctx,
                                size_t index,
                                const uint8_t* data,
                                size_t len)
{
    if (!ctx || !ctx->proc || !data || index >= 32)
        return;
    if (!ctx->proc->any_vector_extensions() || ctx->proc->VU.vlenb == 0)
        return;
    size_t vlenb = static_cast<size_t>(ctx->proc->VU.vlenb);
    if (len != vlenb)
        return;
    char* base = static_cast<char*>(ctx->proc->VU.reg_file);
    memcpy(base + index * vlenb, data, vlenb);
}

void spike_difftest_fini(spike_difftest_ctx_t* ctx)
{
    if (ctx) {
        for (auto& [base, mem] : ctx->simif->mems()) {
            delete mem;
        }
        delete ctx->simif;
        delete ctx->proc;
        delete ctx;
    }
}

} /* extern "C" */

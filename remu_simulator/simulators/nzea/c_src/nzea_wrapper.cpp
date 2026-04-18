// C++ glue for Verilated Nzea models: C API so Rust can create/drive the simulator.
// One Verilated model per (target, isa) tuple from remu nzea/build.rs.
#include "verilated.h"
#include "verilated_fst_c.h"

#include <cstddef>
#include <cstring>
#include <map>

#include "VTop_core_riscv32i.h"
#include "VTop_core_riscv32im.h"
#include "VTop_core_riscv32i_wjCus0.h"
#include "VTop_core_riscv32im_wjCus0.h"
#include "VTop_tile_riscv32i.h"
#include "VTop_tile_riscv32im.h"
#include "VTop_tile_riscv32i_wjCus0.h"
#include "VTop_tile_riscv32im_wjCus0.h"

struct TraceState {
    VerilatedFstC* tfp = nullptr;
    uint64_t time = 0;
};

static std::map<void*, TraceState> s_trace_map;

static bool model_eq(const char* a, const char* b) {
    return a && b && std::strcmp(a, b) == 0;
}

#define NZEA_MODEL_LIST(X)                                    \
    X("core:riscv32i", VTop_core_riscv32i)                  \
    X("core:riscv32im", VTop_core_riscv32im)                \
    X("core:riscv32i_wjCus0", VTop_core_riscv32i_wjCus0)    \
    X("core:riscv32im_wjCus0", VTop_core_riscv32im_wjCus0)  \
    X("tile:riscv32i", VTop_tile_riscv32i)                  \
    X("tile:riscv32im", VTop_tile_riscv32im)                \
    X("tile:riscv32i_wjCus0", VTop_tile_riscv32i_wjCus0)    \
    X("tile:riscv32im_wjCus0", VTop_tile_riscv32im_wjCus0)

extern "C" {

void* nzea_create(const char* model) {
    Verilated::traceEverOn(true);
#define CASE_CREATE(KEY, TYPE) \
    if (model_eq(model, KEY)) { \
        return new TYPE();       \
    }
    NZEA_MODEL_LIST(CASE_CREATE)
#undef CASE_CREATE
    return nullptr;
}

void nzea_destroy(void* sim, const char* model) {
    auto it = s_trace_map.find(sim);
    if (it != s_trace_map.end()) {
        it->second.tfp->close();
        delete it->second.tfp;
        s_trace_map.erase(it);
    }
#define CASE_DESTROY(KEY, TYPE)       \
    if (model_eq(model, KEY)) {       \
        delete static_cast<TYPE*>(sim); \
        return;                       \
    }
    NZEA_MODEL_LIST(CASE_DESTROY)
#undef CASE_DESTROY
}

void nzea_set_clock(void* sim, const char* model, int val) {
#define CASE_SET_CLOCK(KEY, TYPE)    \
    if (model_eq(model, KEY)) {      \
        static_cast<TYPE*>(sim)->clock = val; \
        return;                      \
    }
    NZEA_MODEL_LIST(CASE_SET_CLOCK)
#undef CASE_SET_CLOCK
}

void nzea_set_reset(void* sim, const char* model, int val) {
#define CASE_SET_RESET(KEY, TYPE)    \
    if (model_eq(model, KEY)) {      \
        static_cast<TYPE*>(sim)->reset = val; \
        return;                      \
    }
    NZEA_MODEL_LIST(CASE_SET_RESET)
#undef CASE_SET_RESET
}

void nzea_eval(void* sim, const char* model) {
#define CASE_EVAL(KEY, TYPE)         \
    if (model_eq(model, KEY)) {      \
        static_cast<TYPE*>(sim)->eval(); \
        return;                      \
    }
    NZEA_MODEL_LIST(CASE_EVAL)
#undef CASE_EVAL
}

void nzea_trace_open(void* sim, const char* model, const char* filename) {
    if (s_trace_map.count(sim)) {
        return;
    }

    VerilatedFstC* tfp = new VerilatedFstC();
#define CASE_TRACE(KEY, TYPE)                        \
    if (model_eq(model, KEY)) {                      \
        static_cast<TYPE*>(sim)->contextp()->trace(tfp, 99, 0); \
    } else
    NZEA_MODEL_LIST(CASE_TRACE)
    {
        delete tfp;
        return;
    }
#undef CASE_TRACE

    tfp->open(filename ? filename : "trace.fst");
    tfp->dumpvars(0, "");
    s_trace_map[sim] = {tfp, 0};
}

void nzea_trace_dump(void* sim) {
    auto it = s_trace_map.find(sim);
    if (it == s_trace_map.end()) {
        return;
    }
    it->second.tfp->dump(it->second.time);
    it->second.time++;
}

void nzea_trace_close(void* sim) {
    auto it = s_trace_map.find(sim);
    if (it == s_trace_map.end()) {
        return;
    }
    it->second.tfp->close();
    delete it->second.tfp;
    s_trace_map.erase(it);
}

} // extern "C"

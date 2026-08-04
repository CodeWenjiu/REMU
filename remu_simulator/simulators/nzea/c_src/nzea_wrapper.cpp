// C++ glue for Verilated Nzea models: C API so Rust can create/drive the simulator.
// Built per (target, isa) combination; define NZEA_MODEL_TYPE and NZEA_MODEL_KEY before compiling.
#include "verilated.h"
#include "verilated_fst_c.h"
#include "verilated_vpi.h"

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <map>
#include <string>

#ifndef NZEA_MODEL_TYPE
#error "Define NZEA_MODEL_TYPE to the Verilator class (e.g. -DNZEA_MODEL_TYPE=VTop_core_riscv32i)"
#endif
#ifndef NZEA_MODEL_KEY
#error "Define NZEA_MODEL_KEY to the model key string (e.g. -DNZEA_MODEL_KEY=\\\"core:riscv32i\\\")"
#endif
#ifndef NZEA_MODEL_H
#error "Define NZEA_MODEL_H to the header filename (e.g. -DNZEA_MODEL_H=\\\"VTop_core_riscv32i.h\\\")"
#endif
#include NZEA_MODEL_H

#define NZEA_MODEL_LIST(X) X(NZEA_MODEL_KEY, NZEA_MODEL_TYPE)

struct TraceState {
    VerilatedFstC* tfp = nullptr;
    uint64_t time = 0;
};

static std::map<void*, TraceState> s_trace_map;

static bool model_eq(const char* a, const char* b) {
    return a && b && std::strcmp(a, b) == 0;
}

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

// ---------------------------------------------------------------------------
// VPI signal query
// ---------------------------------------------------------------------------

// Callback invoked for each stat_* counter: leaf name (NUL-terminated) + value.
typedef void (*nzea_stat_callback)(const char* name, uint32_t value, void* userdata);

// Recursively walk module hierarchy, invoking cb for every reg whose leaf name
// starts with "stat_". Returns the number of signals visited in this scope.
// Handles from vpi_scan are managed by Verilator — never vpi_free_object them.
static int collect_stat_signals(vpiHandle scope, nzea_stat_callback cb, void* userdata) {
    int count = 0;
    vpiHandle regs = vpi_iterate(vpiReg, scope);
    if (regs) {
        for (vpiHandle reg = vpi_scan(regs); reg; reg = vpi_scan(regs)) {
            const char* nm = vpi_get_str(vpiName, reg);
            if (!nm || std::strncmp(nm, "stat_", 5) != 0) {
                continue;
            }
            uint32_t value = 0;
            const char* full = vpi_get_str(vpiFullName, reg);
            vpiHandle h =
                full ? vpi_handle_by_name(const_cast<char*>(full), nullptr) : nullptr;
            if (h) {
                s_vpi_value val;
                val.format = vpiIntVal;
                vpi_get_value(h, &val);  // Verilator quirk: first read returns 0
                vpi_get_value(h, &val);
                value = static_cast<uint32_t>(val.value.integer);
            }
            cb(nm, value, userdata);
            count++;
        }
    }
    vpiHandle mods = vpi_iterate(vpiModule, scope);
    if (mods) {
        for (vpiHandle sub = vpi_scan(mods); sub; sub = vpi_scan(mods)) {
            count += collect_stat_signals(sub, cb, userdata);
        }
    }
    return count;
}

// Enumerate all stat_* counters, invoking cb for each. Returns the number
// found, or -1 on error (VPI unavailable / bad args). No capacity or name
// length limits: names are NUL-terminated, count is streamed via the callback.
// Caller must be paused (after continue/step) so values are stable.
int nzea_iter_stats(void* sim, nzea_stat_callback cb, void* userdata) {
    if (!sim || !cb) {
        return -1;
    }
    vpiHandle tops = vpi_iterate(vpiModule, nullptr);
    if (!tops) {
        std::fprintf(stderr, "nzea_iter_stats: VPI unavailable (is --vpi enabled?)\n");
        return -1;
    }
    int count = 0;
    for (vpiHandle top = vpi_scan(tops); top; top = vpi_scan(tops)) {
        count += collect_stat_signals(top, cb, userdata);
    }
    return count;
}

} // extern "C"

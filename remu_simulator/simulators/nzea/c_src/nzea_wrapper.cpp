// C++ glue for Verilated Nzea/Top: C API so Rust can create/drive the sim.
// If your Chisel top is "Nzea", use VNzea.h and VNzea; if it's "Top" (Top.sv), use VTop.
#include "verilated.h"
#include "verilated_fst_c.h"
#include <cstddef>
#include <map>

// Verilator top: Top.sv -> VTop; if your Chisel top is Nzea (Nzea.sv), define NZEA_USE_VNZEA in build.
#if defined(NZEA_USE_VNZEA)
#include "VNzea.h"
#define VNZEA_CLASS VNzea
#else
#include "VTop.h"
#define VNZEA_CLASS VTop
#endif

struct TraceState {
    VerilatedFstC* tfp = nullptr;
    uint64_t time = 0;
};
static std::map<void*, TraceState> s_trace_map;

extern "C" {

void* nzea_create(void) {
    Verilated::traceEverOn(true);
    return new VNZEA_CLASS();
}

void nzea_destroy(void* sim) {
    auto it = s_trace_map.find(sim);
    if (it != s_trace_map.end()) {
        it->second.tfp->close();
        delete it->second.tfp;
        s_trace_map.erase(it);
    }
    delete static_cast<VNZEA_CLASS*>(sim);
}

void nzea_set_clock(void* sim, int val) {
    static_cast<VNZEA_CLASS*>(sim)->clock = val;
}

void nzea_set_reset(void* sim, int val) {
    static_cast<VNZEA_CLASS*>(sim)->reset = val;
}

void nzea_eval(void* sim) {
    static_cast<VNZEA_CLASS*>(sim)->eval();
}

void nzea_trace_open(void* sim, const char* filename) {
    if (s_trace_map.count(sim)) return;
    auto* top = static_cast<VNZEA_CLASS*>(sim);
    auto* tfp = new VerilatedFstC();
    top->contextp()->trace(tfp, 99, 0);  // Must be called before open()
    tfp->open(filename ? filename : "trace.fst");
    tfp->dumpvars(0, "");
    s_trace_map[sim] = {tfp, 0};
}

void nzea_trace_dump(void* sim) {
    auto it = s_trace_map.find(sim);
    if (it == s_trace_map.end()) return;
    it->second.tfp->dump(it->second.time);
    it->second.time++;
}

void nzea_trace_close(void* sim) {
    auto it = s_trace_map.find(sim);
    if (it == s_trace_map.end()) return;
    it->second.tfp->close();
    delete it->second.tfp;
    s_trace_map.erase(it);
}

}

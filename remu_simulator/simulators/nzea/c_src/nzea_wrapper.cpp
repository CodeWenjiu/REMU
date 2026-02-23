// C++ glue for Verilated Nzea/Top: C API so Rust can create/drive the sim.
// If your Chisel top is "Nzea", use VNzea.h and VNzea; if it's "Top" (Top.sv), use VTop.
#include "verilated.h"
#include <cstddef>

// Verilator top: Top.sv -> VTop; if your Chisel top is Nzea (Nzea.sv), define NZEA_USE_VNZEA in build.
#if defined(NZEA_USE_VNZEA)
#include "VNzea.h"
#define VNZEA_CLASS VNzea
#else
#include "VTop.h"
#define VNZEA_CLASS VTop
#endif

extern "C" {

void* nzea_create(void) {
    Verilated::traceEverOn(false);
    return new VNZEA_CLASS();
}

void nzea_destroy(void* sim) {
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

}

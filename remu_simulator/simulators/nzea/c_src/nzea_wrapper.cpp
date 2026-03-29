// C++ glue for Verilated Nzea/Top: C API so Rust can create/drive the sim.
// One Verilated model per `NZEA_ISAS` entry in build.rs (each has its own VTop_<isa> class).
#include "verilated.h"
#include "verilated_fst_c.h"
#include <cstddef>
#include <map>
#include <cstring>

#include "VTop_riscv32i.h"
#include "VTop_riscv32im.h"
#include "VTop_riscv32i_wjCus0.h"
#include "VTop_riscv32im_wjCus0.h"

struct TraceState {
    VerilatedFstC* tfp = nullptr;
    uint64_t time = 0;
};
static std::map<void*, TraceState> s_trace_map;

static bool isa_eq(const char* a, const char* b) {
    return a && b && strcmp(a, b) == 0;
}

extern "C" {

void* nzea_create(const char* isa) {
    Verilated::traceEverOn(true);
    if (isa_eq(isa, "riscv32i")) {
        return new VTop_riscv32i();
    }
    if (isa_eq(isa, "riscv32im")) {
        return new VTop_riscv32im();
    }
    if (isa_eq(isa, "riscv32i_wjCus0")) {
        return new VTop_riscv32i_wjCus0();
    }
    if (isa_eq(isa, "riscv32im_wjCus0")) {
        return new VTop_riscv32im_wjCus0();
    }
    return nullptr;
}

void nzea_destroy(void* sim, const char* isa) {
    auto it = s_trace_map.find(sim);
    if (it != s_trace_map.end()) {
        it->second.tfp->close();
        delete it->second.tfp;
        s_trace_map.erase(it);
    }
    if (isa_eq(isa, "riscv32i")) {
        delete static_cast<VTop_riscv32i*>(sim);
    } else if (isa_eq(isa, "riscv32im")) {
        delete static_cast<VTop_riscv32im*>(sim);
    } else if (isa_eq(isa, "riscv32i_wjCus0")) {
        delete static_cast<VTop_riscv32i_wjCus0*>(sim);
    } else if (isa_eq(isa, "riscv32im_wjCus0")) {
        delete static_cast<VTop_riscv32im_wjCus0*>(sim);
    }
}

void nzea_set_clock(void* sim, const char* isa, int val) {
    if (isa_eq(isa, "riscv32i")) {
        static_cast<VTop_riscv32i*>(sim)->clock = val;
    } else if (isa_eq(isa, "riscv32im")) {
        static_cast<VTop_riscv32im*>(sim)->clock = val;
    } else if (isa_eq(isa, "riscv32i_wjCus0")) {
        static_cast<VTop_riscv32i_wjCus0*>(sim)->clock = val;
    } else if (isa_eq(isa, "riscv32im_wjCus0")) {
        static_cast<VTop_riscv32im_wjCus0*>(sim)->clock = val;
    }
}

void nzea_set_reset(void* sim, const char* isa, int val) {
    if (isa_eq(isa, "riscv32i")) {
        static_cast<VTop_riscv32i*>(sim)->reset = val;
    } else if (isa_eq(isa, "riscv32im")) {
        static_cast<VTop_riscv32im*>(sim)->reset = val;
    } else if (isa_eq(isa, "riscv32i_wjCus0")) {
        static_cast<VTop_riscv32i_wjCus0*>(sim)->reset = val;
    } else if (isa_eq(isa, "riscv32im_wjCus0")) {
        static_cast<VTop_riscv32im_wjCus0*>(sim)->reset = val;
    }
}

void nzea_eval(void* sim, const char* isa) {
    if (isa_eq(isa, "riscv32i")) {
        static_cast<VTop_riscv32i*>(sim)->eval();
    } else if (isa_eq(isa, "riscv32im")) {
        static_cast<VTop_riscv32im*>(sim)->eval();
    } else if (isa_eq(isa, "riscv32i_wjCus0")) {
        static_cast<VTop_riscv32i_wjCus0*>(sim)->eval();
    } else if (isa_eq(isa, "riscv32im_wjCus0")) {
        static_cast<VTop_riscv32im_wjCus0*>(sim)->eval();
    }
}

void nzea_trace_open(void* sim, const char* isa, const char* filename) {
    if (s_trace_map.count(sim)) return;
    VerilatedFstC* tfp = new VerilatedFstC();
    if (isa_eq(isa, "riscv32i")) {
        static_cast<VTop_riscv32i*>(sim)->contextp()->trace(tfp, 99, 0);
    } else if (isa_eq(isa, "riscv32im")) {
        static_cast<VTop_riscv32im*>(sim)->contextp()->trace(tfp, 99, 0);
    } else if (isa_eq(isa, "riscv32i_wjCus0")) {
        static_cast<VTop_riscv32i_wjCus0*>(sim)->contextp()->trace(tfp, 99, 0);
    } else if (isa_eq(isa, "riscv32im_wjCus0")) {
        static_cast<VTop_riscv32im_wjCus0*>(sim)->contextp()->trace(tfp, 99, 0);
    } else {
        delete tfp;
        return;
    }
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

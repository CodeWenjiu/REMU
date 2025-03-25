#include <stdint.h>
#include <stdbool.h>

typedef struct {
    uint32_t gpr[32];
    uint32_t pc;
} riscv32_CPU_state;

void difftest_init(int port);
void difftest_memcpy(uint32_t addr, void *buf, uint64_t n, bool direction);
void difftest_regcpy(void *dut, bool direction);
void difftest_exec(uint64_t n);
void difftest_raise_intr(uint64_t NO);

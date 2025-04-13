Job = -j `nproc`

Binfile_Nzea = ./.test/microbench-riscv32e-npc.bin
Binfile_jyd = ./.test/jyd_driver-riscv32e-npc.bin
Binfile_Emu = ./.test/microbench-riscv32-nemu.bin

Platform_emu_rv32im = rv32im-emu
Platform_emu_rv32e = rv32e-emu
Platform_emu_default = $(Platform_emu_rv32im)

Platform_Nzea = rv32e-nzea

Binfile_default = $(Binfile_Emu)
Platform_default = $(Platform_emu_rv32im)

Mainargs = --bin $(Binfile_default) -p $(Platform_default)
Debugargs = $(Mainargs) # -d emu--log

default: run

menuconfig:
	@$(MAKE) -C ./config menuconfig

clean: 
	@$(MAKE) -C ./config clean

run :
	@cargo run $(Job) --release --bin core -- $(Mainargs)

debug :
	@RUST_BACKTRACE=full cargo run $(Job) --bin core -- $(Debugargs)

.PHONY: default menuconfig clean run debug


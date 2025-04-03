Job = -j `nproc`

Binfile_Emu = ./.test/microbench-riscv32-nemu.bin
Binfile_Nzea = ./.test/microbench-riscv32e-npc.bin

Platform_emu_rv32im = rv32im-emu
Platform_emu_rv32e = rv32e-emu
Platform_emu_default = $(Platform_emu_rv32im)

Platform_nzea = rv32e-nzea

Binfile_default = $(Binfile_Nzea)
Platform_default = $(Platform_nzea)

Mainargs = --bin $(Binfile_default) -p $(Platform_default)
Debugargs = $(Mainargs) -d spike #--log

default: run

menuconfig:
	@$(MAKE) -C ./config menuconfig

clean: 
	@$(MAKE) -C ./config clean

run :
	cargo run $(Job) --release --bin core -- $(Mainargs)

debug :
	RUST_BACKTRACE=full cargo run $(Job) --bin core -- $(Debugargs)

.PHONY: default menuconfig clean run debug


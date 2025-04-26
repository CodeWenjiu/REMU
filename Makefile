Job = -j `nproc`

Binfile_Nzea_npc = ./.test/microbench-riscv32e-npc.bin
Binfile_Nzea_ysyxsoc = ./.test/microbench-riscv32e-ysyxsoc.bin
Binfile_jyd = ./.test/jyd_driver-riscv32e-npc.bin
Binfile_Emu = ./.test/microbench-riscv32-nemu.bin

Platform_emu_rv32im = rv32im-emu-nemu
Platform_emu_rv32e = rv32e-emu-nemu
Platform_emu_default = $(Platform_emu_rv32im)

Platform_Nzea_npc = rv32e-nzea-npc
Platform_Nzea_ysyxsoc = rv32e-nzea-ysyxsoc

Platform ?= $(Platform_Nzea)
# Set Binfile based on Platform
ifeq ($(Platform),$(Platform_emu_rv32im))
	Binfile ?= $(Binfile_Emu)
else ifeq ($(Platform),$(Platform_emu_rv32e))
	Binfile ?= $(Binfile_Emu)
else ifeq ($(Platform),$(Platform_Nzea_ysyxsoc))
	Binfile ?= $(Binfile_Nzea_ysyxsoc)
else
	Binfile ?= $(Binfile_Nzea)
endif

Mainargs = --bin $(Binfile) -p $(Platform)
ExtraArgs ?=
Debugargs = $(Mainargs) # -d emu--log

default: run

menuconfig:
	@$(MAKE) -C ./config menuconfig

clean: 
	@$(MAKE) -C ./config clean

run :
	@cargo run $(Job) --release --bin core -- $(Mainargs) $(ExtraArgs)

debug :
	@RUST_BACKTRACE=full cargo run $(Job) --bin core -- $(Debugargs)

fmt :
	@cargo fmt --all 

.PHONY: default menuconfig clean run debug fmt


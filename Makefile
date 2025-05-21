Job = -j `nproc`

Binfile_Nzea_npc = ./.test/microbench-riscv32e-npc.bin

Binfile_Nzea_ysyxsoc = ./.test/microbench-riscv32e-ysyxsoc.bin

Binfile_jyd = ./.test/jyd_driver-riscv32e-npc.bin

Binfile_jyd_remote = ./simulator/src/nzea/on_board/NZ-jyd/tools/bin_spliter/irom.bin
Alternate_jyd_remote = 0x80100000:./simulator/src/nzea/on_board/NZ-jyd/tools/bin_spliter/dram.bin

Binfile_Emu = ./.test/image.bin

Platform_emu_rv32im = rv32im-emu-nemu
Platform_emu_rv32e = rv32e-emu-nemu
Platform_emu_default = $(Platform_emu_rv32im)

Platform_Nzea_npc = rv32e-nzea-npc
Platform_Nzea_ysyxsoc = rv32e-nzea-ysyxsoc
Platform_Nzea_jyd_remote = rv32i-nzea-jyd_remote

Platform ?= $(Platform_emu_default)

PLATFORMS = $(Platform_emu_rv32im) $(Platform_emu_rv32e) $(Platform_Nzea_npc) $(Platform_Nzea_ysyxsoc) $(Platform_Nzea_jyd_remote)

ifeq ($(filter clean menuconfig fmt,$(MAKECMDGOALS)),)

ifeq ($(filter $(PLATFORMS), $(Platform)), )
$(error Expected $$PLATFORM in {$(PLATFORMS)}, Got "$(Platform)")
endif

endif

# Set Binfile based on Platform
ifeq ($(Platform),$(Platform_emu_rv32im))
	Binfile ?= $(Binfile_Emu)
else ifeq ($(Platform),$(Platform_emu_rv32e))
	Binfile ?= $(Binfile_Emu)
else ifeq ($(Platform),$(Platform_Nzea_npc))
	Binfile ?= $(Binfile_Nzea_npc)
else ifeq ($(Platform),$(Platform_Nzea_ysyxsoc))
	Binfile ?= $(Binfile_Nzea_ysyxsoc)
else ifeq ($(Platform),$(Platform_Nzea_jyd_remote))
	Binfile ?= $(Binfile_jyd_remote)
	Alternate ?= --additional-bin $(Alternate_jyd_remote)
else
	Binfile ?= $(Binfile_Nzea)
endif

Mainargs = --primary-bin $(Binfile) $(Alternate) -p $(Platform)
ExtraArgs ?=
Debugargs = $(Mainargs) -d emu #--log

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


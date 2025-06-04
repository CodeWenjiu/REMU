Job = -j `nproc`

Binfile_Nzea_npc = ./.test/microbench-riscv32e-npc.bin

Binfile_Nzea_ysyxsoc = ./.test/microbench-riscv32e-ysyxsoc.bin

Binfile_jyd = ./.test/jyd_driver-riscv32e-npc.bin

Binfile_jyd_remote = ./simulator/src/nzea/on_board/NZ-jyd/tools/bin_spliter/irom.bin
Alternate_jyd_remote = 0x80100000:./simulator/src/nzea/on_board/NZ-jyd/tools/bin_spliter/dram.bin

Binfile_Emu = ./.test/microbench-riscv32-nemu.bin

Platform_rv32im_emu_dm = rv32im-emu-dm
Platform_rv32im_emu_dm_alias = riscv32-emu-dm

Platform_rv32im_emu_sc = rv32im-emu-sc
Platform_rv32im_emu_sc_alias = riscv32-emu-sc

Platform_rv32e_emu = rv32e-emu-dm
Platform_emu_default = $(Platform_rv32im_emu_dm)

Platform_Nzea_npc = rv32e-nzea-npc
Platform_Nzea_ysyxsoc = rv32e-nzea-ysyxsoc
Platform_Nzea_jyd_remote = rv32i-nzea-jyd_remote

Platform ?= $(Platform_emu_default)

PLATFORMS = $(Platform_rv32im_emu_dm) $(Platform_rv32im_emu_dm_alias) $(Platform_rv32im_emu_sc) $(Platform_rv32im_emu_sc_alias) $(Platform_rv32e_emu) $(Platform_Nzea_npc) $(Platform_Nzea_ysyxsoc) $(Platform_Nzea_jyd_remote)

ifeq ($(filter clean menuconfig fmt,$(MAKECMDGOALS)),)

ifeq ($(filter $(PLATFORMS), $(Platform)), )
$(error Expected $$PLATFORM in {$(PLATFORMS)}, Got "$(Platform)")
endif

endif

ifeq ($(Platform),$(Platform_rv32im_emu_dm))
    Binfile ?= $(Binfile_Emu)
else ifeq ($(Platform),$(Platform_rv32im_emu_dm_alias))
    Binfile ?= $(Binfile_Emu)
else ifeq ($(Platform),$(Platform_rv32im_emu_sc))
    Binfile ?= $(Binfile_Emu)
else ifeq ($(Platform),$(Platform_rv32im_emu_sc_alias))
    Binfile ?= $(Binfile_Emu)
else ifeq ($(Platform),$(Platform_rv32e_emu))
    Binfile ?= $(Binfile_Emu)
else ifeq ($(Platform),$(Platform_Nzea_npc))
    Binfile ?= $(Binfile_Nzea_npc)
else ifeq ($(Platform),$(Platform_Nzea_ysyxsoc))
    Binfile ?= $(Binfile_Nzea_ysyxsoc)
else ifeq ($(Platform),$(Platform_Nzea_jyd_remote))
    Binfile ?= $(Binfile_jyd_remote)
    Alternate ?= --additional-bin $(Alternate_jyd_remote)
else
    $(info No match found)
    Binfile ?= $(Binfile_Nzea)
endif

$(info Final Binfile = $(Binfile))

Mainargs = --primary-bin $(Binfile) $(Alternate) -p $(Platform)
ExtraArgs ?=
Debugargs = $(Mainargs) -d emu #--log

default: print_binfile run

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


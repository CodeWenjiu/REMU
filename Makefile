# ==============================================================================
# Build Configuration
# ==============================================================================

Job = -j `nproc`

# ==============================================================================
# Binary Files Configuration
# ==============================================================================

Binfile_jyd_remote = ./simulator/src/nzea/on_board/NZ-jyd/tools/bin_spliter/irom.bin
Alternate_jyd_remote = 0x80100000:./simulator/src/nzea/on_board/NZ-jyd/tools/bin_spliter/dram.bin

# ==============================================================================
# Platform Definitions
# ==============================================================================

# EMU Platforms
Platform_rv32im_emu_dm       = rv32im-emu-dm
Platform_rv32im_emu_dm_alias = riscv32-emu-dm

Platform_rv32im_emu_sc       = rv32im-emu-sc
Platform_rv32im_emu_sc_alias = riscv32-emu-sc

Platform_rv32im_emu_pl       = rv32im-emu-pl
Platform_rv32im_emu_pl_alias = riscv32-emu-pl

Platform_rv32e_emu           = rv32e-emu-dm

# NZEA Platforms
Platform_Nzea_npc        = rv32e-nzea-npc
Platform_Nzea_npc_alias  = riscv32e-nzea-npc

Platform_Nzea_ysyxsoc    		= rv32e-nzea-ysyxsoc
Platform_Nzea_ysyxsoc_alias    	= riscv32e-nzea-ysyxsoc

Platform_Nzea_jyd_remote = rv32i-nzea-jyd_remote

# Default Platform
Platform_emu_default = $(Platform_rv32im_emu_dm)
Platform ?= $(Platform_emu_default)

# All Supported Platforms
PLATFORMS = \
	$(Platform_rv32im_emu_dm) $(Platform_rv32im_emu_dm_alias) \
	$(Platform_rv32im_emu_sc) $(Platform_rv32im_emu_sc_alias) \
	$(Platform_rv32im_emu_pl) $(Platform_rv32im_emu_pl_alias) \
	$(Platform_rv32e_emu) \
	$(Platform_Nzea_npc) $(Platform_Nzea_npc_alias) \
	$(Platform_Nzea_ysyxsoc) $(Platform_Nzea_ysyxsoc_alias)\
	$(Platform_Nzea_jyd_remote)

# ==============================================================================
# Configuration Files
# ==============================================================================

ConfigFile = config/dynamic/.config

# Platform validation (skip for specific targets)
ifeq ($(filter clean menuconfig fmt,$(MAKECMDGOALS)),)
ifeq ($(filter $(PLATFORMS), $(Platform)), )
$(error Expected $$PLATFORM in {$(PLATFORMS)}, Got "$(Platform)")
endif
endif

# ==============================================================================
# Binary Command Configuration
# ==============================================================================

BinCommand ?= $(if $(Binfile),--primary-bin $(abspath $(Binfile)),)
AdditionalBinCommand ?= $(if $(Alternate),--additional-bin $(abspath $(Alternate)),)

# Platform-specific binary configuration
ifeq ($(Platform),$(Platform_Nzea_jyd_remote))
	Binfile ?= $(Binfile_jyd_remote)
	Alternate ?= $(Alternate_jyd_remote)
endif

# ==============================================================================
# Difftest Configuration
# ==============================================================================

Default_FFI_Path = $(abspath ./remu_buildin/difftest_ref)
Difftest_FFI_Spike = $(Default_FFI_Path)/riscv32-spike-so

emu_SingleCycle = emu-sc
emu_Pipeline = emu-pl

DifftestArgs = -d $(emu_SingleCycle)

# ==============================================================================
# Main Arguments
# ==============================================================================

Mainargs = \
	$(BinCommand) $(AdditionalBinCommand) \
	-p $(Platform) \
	-c $(abspath $(ConfigFile)) \
	$(DifftestArgs)

ExtraComand ?= $(if $(ExtraCmd),-e $(ExtraCmd), )
Debugargs = $(Mainargs) #--log

# ==============================================================================
# Targets
# ==============================================================================

default: run

# Configuration Targets
menuconfig-static:
	@$(MAKE) -C ./config menuconfig-static

menuconfig-dynamic:
	@$(MAKE) -C ./config menuconfig-dynamic

menuconfig:
	@$(MAKE) -C ./config menuconfig

menuconfig-static-conditional:
	@$(MAKE) -C ./config menuconfig-static-conditional

menuconfig-dynamic-conditional:
	@$(MAKE) -C ./config menuconfig-dynamic-conditional

config_dependencies: menuconfig-static-conditional menuconfig-dynamic-conditional

# Clean Targets
clean:
	@cargo clean

clean-config:
	@$(MAKE) -C ./config clean

clean-all: clean clean-config

# Build and Run Targets
run: config_dependencies
	@echo $(ExtraCmd)
	@cargo run $(Job) --release --bin core -- $(Mainargs) $(ExtraComand)

DEFAULT_PERF_PATH = $(abspath ../am-kernels/benchmarks/microbench/)

perf: 
	@$(MAKE) -C $(DEFAULT_PERF_PATH) ARCH=riscv32e-ysyxsoc mainargs=test perf

debug: config_dependencies
	@RUST_BACKTRACE=full cargo run $(Job) --bin core -- $(Debugargs) $(ExtraComand)

# Development Targets
fmt:
	@cargo fmt --all --manifest-path ./Cargo.toml

# Utility Targets
fetch:
	@onefetch

CommitMsg = commitmsg.txt

$(CommitMsg):
	git log --pretty=format:"%at|%s" --reverse --no-merges > commitmsg.txt

gource: $(CommitMsg)
	@gource --load-config ./gource.ini

CHECK_HASH ?=

checkout:
	@git checkout $(CHECK_HASH)
	@git submodule update --recursive

# ==============================================================================
# Phony Targets
# ==============================================================================

.PHONY: default config_dependencies \
	menuconfig-static menuconfig-dynamic menuconfig \
	clean clean-config clean-all \
	run perf debug fmt fetch gource

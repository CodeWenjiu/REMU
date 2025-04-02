Job = -j `nproc`

Binfile = ./.test/microbench-riscv32e-npc.bin

Mainargs = --bin $(Binfile) -p rv32e-emu
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


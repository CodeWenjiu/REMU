Job = -j `nproc`

Binfile = ./.test/microbench-riscv32-nemu.bin

Mainargs = --bin $(Binfile) -p rv32e-nzea
Debugargs = $(Mainargs) #--log -d spike

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


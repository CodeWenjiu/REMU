Job = -j `nproc`

Binfile = ./.test/microbench-riscv32-nemu.bin

Mainargs = --bin $(Binfile)
Debugargs = $(Mainargs) --log -d spike

menuconfig:
	@$(MAKE) -C ./config menuconfig

clean: 
	@$(MAKE) -C ./config clean

run :
	cargo run $(Job) --release --bin core -- $(Mainargs)

debug :
	RUST_BACKTRACE=full cargo run $(Job) --bin core -- $(Debugargs)

.PHONY: menuconfig clean run


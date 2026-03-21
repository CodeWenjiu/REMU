_default:
    @just --list

dev *args:
    @RUST_BACKTRACE=1 cargo run -p remu_cli -- {{ args }}

build:
    @cargo build -p remu_cli

run *args:
    @RUST_BACKTRACE=1 cargo run -p remu_cli --release -- {{ args }}

bench CRATE BENCH:
    @cargo bench -p remu_{{ CRATE }} --bench {{ BENCH }}

flame CRATE BENCH:
    @cargo bench -p remu_{{ CRATE }} --bench {{ BENCH }} -- --profile-time 20

clean-all:
    @cargo clean

# Build remu-app (release). Output: target/app/<triple>/release/
# TARGET: ISA shorthand (riscv32i, riscv32im, riscv32imac) or full triple.
# Requires cargo-binutils. Run `just clean-app` to remove target/app/
build-app APP target="riscv32i":
    cargo run -p xtask -- build-app {{ APP }} {{ target }}

# Run remu-app on remu: build + load ELF + run to exit.
# Uses Cargo runner (.cargo/config.toml). TARGET: riscv32i, riscv32im, riscv32imac.
run-app APP target="riscv32i":
    @cargo run -p remu_app_{{ APP }} --target {{ target }}-unknown-none-elf --release -Z build-std=core,alloc

# Remove all embedded app build artifacts (target/app/). Does not affect remu_cli etc.
clean-app:
    @rm -rf "{{ justfile_directory() }}/target/app"


look:
    @cargo asm --release -p remu_cli run_steps
    
step-sizes:
    @cargo build --profile bench -p remu_cli
    @nm -S -C "{{ justfile_directory() }}/target/release/remu_cli" 2>&1 | grep run_steps | gawk 'BEGIN { OFS="\t"; print "size_hex\tsize_bytes\tvariant" } { size_hex=$2; size_dec=strtonum("0x"$2); rest=$0; sub(/^[^ \t]+[ \t]+[^ \t]+[ \t][ \t]*/, "", rest); idx=index(rest, "Debugger<"); end=index(rest, ">>::"); if (idx && end) variant=substr(rest, idx+9, end-idx-9); else variant=rest; gsub(/remu_types::isa::extension_enum::/, "", variant); gsub(/remu_state::policy::/, "", variant); print size_hex, size_dec, variant }' > "{{ justfile_directory() }}/.step-sizes.tsv"
    @bash -c 'test $(wc -l < "{{ justfile_directory() }}/.step-sizes.tsv") -gt 1 || { echo "No run_steps symbols. Run: cargo build --profile bench -p remu_cli  then: nm -S -C target/release/remu_cli | grep run_steps"; exit 1; }'
    @nu -c 'open "{{ justfile_directory() }}/.step-sizes.tsv" --raw | from tsv | update size_bytes { |r| $r.size_bytes | into int } | table -e'

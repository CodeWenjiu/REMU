_default:
    @just --list

dev *args:
    @RUST_BACKTRACE=1 cargo run -p remu_cli -- {{ args }}

build:
    @cargo build -p remu_cli

run *args:
    @RUST_BACKTRACE=1 cargo run -p remu_cli --release -- {{ args }}

clean-all:
    @cargo clean

build-app APP target="riscv32i":
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ justfile_directory() }}"
    eval "$(cargo run -p xtask -- print build-app "{{ APP }}" "{{ target }}")"

[arg("app_args", long="app-args")]
[arg("dev", long="dev", value="1")]
[arg("platform", long="platform")]
run-app APP target="riscv32i" platform="remu" dev='' app_args='' *remu_cli_args:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ justfile_directory() }}"
    export REMU_APP_ARGS="{{ app_args }}"
    if [ "{{ platform }}" = "host" ]; then
        cargo run --release -p "remu_app_{{ APP }}" -- {{ app_args }}
    elif [ "{{ platform }}" = "qemu" ]; then
        eval "$(cargo run -p xtask -- print build-app "{{ APP }}" "{{ target }}" --platform qemu)"
        ELF="target/app/{{ target }}-unknown-none-elf/release/remu_app_{{ APP }}"
        qemu-system-riscv32 -M virt -m 256M -nographic -bios none -kernel "$ELF"
    elif [ "{{ platform }}" = "spike" ]; then
        eval "$(cargo run -p xtask -- print build-app "{{ APP }}" "{{ target }}" --platform spike)"
        ELF="target/app/{{ target }}-unknown-none-elf/release/remu_app_{{ APP }}"
        SPIKE_ISA="$(echo {{ target }} | sed 's/^riscv/RV/' | tr '[:lower:]' '[:upper:]')"
        # --real-time-clint: mtime tracks wall clock (like remu's CLINT), so
        # guest-reported times / microbench scores are comparable across platforms.
        spike --real-time-clint --isa "$SPIKE_ISA" -m0x80000000:0x08000000 "$ELF"
    else
        {{ if dev != '' { "export DEV=1;" } else { "" } }}
        eval "$(cargo run -p xtask -- print run-app "{{ APP }}" "{{ target }}" --platform {{ platform }} -- {{ remu_cli_args }})"
    fi

clean-app:
    @rm -rf "{{ justfile_directory() }}/target/app" "{{ justfile_directory() }}/target/app_zve32x"

look:
    @cargo asm --release -p remu_cli run_steps

step-sizes:
    @cargo build --profile bench -p remu_cli
    @nm -S -C "{{ justfile_directory() }}/target/release/remu_cli" 2>&1 | grep run_steps | gawk 'BEGIN { OFS="\t"; print "size_hex\tsize_bytes\tvariant" } { size_hex=$2; size_dec=strtonum("0x"$2); rest=$0; sub(/^[^ \t]+[ \t]+[^ \t]+[ \t][ \t]*/, "", rest); idx=index(rest, "Debugger<"); end=index(rest, ">>::"); if (idx && end) variant=substr(rest, idx+9, end-idx-9); else variant=rest; gsub(/remu_types::isa::extension_enum::/, "", variant); gsub(/remu_state::policy::/, "", variant); print size_hex, size_dec, variant }' > "{{ justfile_directory() }}/.step-sizes.tsv"
    @bash -c 'test $(wc -l < "{{ justfile_directory() }}/.step-sizes.tsv") -gt 1 || { echo "No run_steps symbols. Run: cargo build --profile bench -p remu_cli  then: nm -S -C target/release/remu_cli | grep run_steps"; exit 1; }'
    @nu -c 'open "{{ justfile_directory() }}/.step-sizes.tsv" --raw | from tsv | update size_bytes { |r| $r.size_bytes | into int } | table -e'

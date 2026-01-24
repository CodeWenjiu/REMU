_default:
    @just --list

dev *args:
    @cargo run {{ args }}

build:
    @cargo build

run *args:
    @cargo run --release {{ args }}

bench CRATE BENCH:
    @cargo bench -p remu_{{ CRATE }} --bench {{ BENCH }}

flame CRATE BENCH:
    @cargo bench -p remu_{{ CRATE }} --bench {{ BENCH }} -- --profile-time 20

clean:
    @cargo clean

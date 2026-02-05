_default:
    @just --list

dev *args:
    @RUST_BACKTRACE=1 cargo run -- {{ args }}

build:
    @cargo build -p remu_cli

run *args:
    @RUST_BACKTRACE=1 cargo run --release -- {{ args }}

bench CRATE BENCH:
    @cargo bench -p remu_{{ CRATE }} --bench {{ BENCH }}

flame CRATE BENCH:
    @cargo bench -p remu_{{ CRATE }} --bench {{ BENCH }} -- --profile-time 20

clean:
    @cargo clean

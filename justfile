_default:
    @just --list

dev *args:
    @cargo run {{ args }}

build:
    @cargo build

run *args:
    @cargo run --release {{ args }}

bench BENCH:
    @cargo bench -p remu_state --bench {{ BENCH }}

flame BENCH:
    @cargo bench -p remu_state --bench {{ BENCH }} -- --profile-time 20

clean:
    @cargo clean

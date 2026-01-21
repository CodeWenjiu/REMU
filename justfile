_default:
    @just --list

dev *args:
    @cargo run {{ args }}

build:
    @cargo build

run *args:
    @cargo run --release {{ args }}

bench:
    @cargo bench -p remu_state --bench bus_read

flame:
    @cargo bench -p remu_state --bench bus_read -- --profile-time 20

clean:
    @cargo clean

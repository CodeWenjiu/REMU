_default:
    @just --list

dev *args:
    @cargo run {{ args }}

build:
    @cargo build

run *args:
    @cargo run --release {{ args }}

bench:
    @cargo bench --profile release

clean:
    @cargo clean

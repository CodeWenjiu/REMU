_default:
    @just --list

run *args:
    @cargo run --release {{args}}

dev *args:
    @cargo run {{args}}

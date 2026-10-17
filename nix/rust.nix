# Rust toolchain for the remu workspace (nightly + the bare-metal targets).
{ pkgs }:

pkgs.rust-bin.nightly.latest.default.override {
  extensions = [
    "rust-src"
    "clippy"
    "rust-analyzer"
    "llvm-tools-preview"
    "rustc-codegen-cranelift-preview"
  ];
  targets = [
    "riscv32i-unknown-none-elf"
    "riscv32im-unknown-none-elf"
    "riscv32imac-unknown-none-elf"
  ];
}
extern crate bindgen;

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::copy("/home/wenjiu/ysyx-workbench/nemu/tools/spike-diff/build/riscv32-spike-so", out_dir.join("libriscv32-spike.so")).unwrap();

    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rustc-link-lib=riscv32-spike");

    let builder = bindgen::Builder::default()
        .header("difftest_ffi.h")
        .clang_arg("-Isrc")
        .generate_comments(false);

    let bindings = builder.generate().expect("Failed to generate bindings");

    bindings
        .write_to_file("ffi/bindings.rs")
        .expect("Failed to write bindings");
}

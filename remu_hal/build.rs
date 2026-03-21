// Make memory.x visible to the linker for RISC-V bare-metal builds.
fn main() {
    println!("cargo:rustc-link-search={}", std::env::var("CARGO_MANIFEST_DIR").unwrap());
}

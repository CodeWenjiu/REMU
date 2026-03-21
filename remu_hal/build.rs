// Make memory.x visible to the linker for RISC-V bare-metal builds.
// -L adds our dir so -Tmemory.x (from config) finds memory.x; link.x from riscv-rt.
fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search={manifest}");
}

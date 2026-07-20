use std::io::Write;

// Make memory.x visible to the linker for RISC-V bare-metal builds.
// -L adds our dir so -Tmemory.x (from config) finds memory.x; link.x from riscv-rt.
fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search={manifest}");
    for p in xtask::Platform::names() {
        println!("cargo::rustc-check-cfg=cfg(platform_{p})");
    }

    // Embed application arguments into the firmware binary.
    let app_args = std::env::var("REMU_APP_ARGS").unwrap_or_default();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let path = std::path::Path::new(&out_dir).join("app_args_data.rs");
    let mut f = std::fs::File::create(&path).unwrap();

    let bytes: Vec<u8> = app_args
        .as_bytes()
        .iter()
        .copied()
        .chain(std::iter::once(0))
        .take(4096)
        .collect();

    write!(f, "&[").unwrap();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            write!(f, ", ").unwrap();
        }
        write!(f, "0x{b:02x}u8").unwrap();
    }
    writeln!(f, "]").unwrap();

    println!("cargo:rerun-if-env-changed=REMU_APP_ARGS");
}

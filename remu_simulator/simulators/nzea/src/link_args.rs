pub fn emit_linker_args() {
    let list = format!("{}/nzea_symbols.list", env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rustc-link-arg=-Wl,--dynamic-list={list}");
}

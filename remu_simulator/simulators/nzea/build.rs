fn main() {
    let zlib_lib = std::process::Command::new("pkg-config")
        .args(["--variable=libdir", "zlib"])
        .output()
        .ok()
        .and_then(|o| {
            let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !path.is_empty() { Some(path) } else { None }
        })
        .unwrap_or_else(|| "/usr/lib".to_string());
    println!("cargo:rustc-env=NZEA_ZLIB_LIB_DIR={zlib_lib}");
}

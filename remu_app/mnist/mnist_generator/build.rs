//! Compiles the generator's `.slint` UI.
//!
//! Same setup as `remu_app/slint`: resources are embedded for the software
//! renderer, and Slint's compile-time default-font lookup is pointed at a real
//! font file because the nix dev shell's font paths are not visible to
//! fontique.

fn main() {
    if std::env::var("SLINT_DEFAULT_FONT").is_err()
        && let Some(font) = find_default_font()
    {
        // SAFETY: setting an env var in a build script before running the
        // slint compiler is single-threaded and fine.
        unsafe {
            std::env::set_var("SLINT_DEFAULT_FONT", &font);
        }
    }

    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    slint_build::compile_with_config("ui/mnist_draw.slint", config)
        .expect("slint compilation failed");
    println!("cargo:rerun-if-changed=ui/mnist_draw.slint");
}

/// Locate a usable sans-serif .ttf for the Slint compiler.
fn find_default_font() -> Option<std::path::PathBuf> {
    // 1. Ask fontconfig for a sans-serif font.
    if let Ok(out) = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", "sans-serif"])
        .output()
        && out.status.success()
    {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() && std::path::Path::new(&path).exists() {
            return Some(path.into());
        }
    }
    // 2. Fall back to common font locations.
    for p in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ] {
        let path = std::path::Path::new(p);
        if path.is_file() {
            return Some(path.to_path_buf());
        }
    }
    None
}

//! Compiles the app's `.slint` UI.
//!
//! Fonts/images are always embedded for the software renderer (host and
//! embedded both render with Slint's software renderer via remu_hal_slint).
//!
//! Slint looks up a default sans-serif font at compile time from the system
//! (fontconfig) or from `SLINT_DEFAULT_FONT`. In the nix dev shell the system
//! font paths are not visible to fontique, so we point SLINT_DEFAULT_FONT at a
//! font file explicitly. On systems where fontconfig already resolves fonts,
//! this is a harmless override that just picks one font.

fn main() {
    if std::env::var("SLINT_DEFAULT_FONT").is_err() {
        if let Some(font) = find_default_font() {
            // SAFETY: setting an env var in a build script before running the
            // slint compiler is single-threaded and fine.
            unsafe {
                std::env::set_var("SLINT_DEFAULT_FONT", &font);
            }
        }
    }

    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    slint_build::compile_with_config("ui/main.slint", config).expect("slint compilation failed");
    println!("cargo:rerun-if-changed=ui/main.slint");
}

/// Locate a usable sans-serif .ttf for the Slint compiler.
fn find_default_font() -> Option<std::path::PathBuf> {
    // 1. Ask fontconfig for a sans-serif font.
    if let Ok(out) = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", "sans-serif"])
        .output()
    {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() && std::path::Path::new(&path).exists() {
                return Some(path.into());
            }
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

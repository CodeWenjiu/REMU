//! Lazy build & load of libspike.so at runtime.
//!
//! On first use, hashes spike source files; if the hash changed or the .so
//! is missing, runs configure + make + g++ -shared to produce a fresh
//! libspike.so.  Then dlopen's it via libloading and caches the function
//! pointers in a static OnceLock.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use libloading::Library;
use sha2::{Digest, Sha256};

use crate::ffi::SpikeFns;

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

fn spike_src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spike")
}

/// Directory where libspike.so and its stamp file live.
fn so_dir() -> PathBuf {
    out_dir().join("remu-so")
}

fn out_dir() -> PathBuf {
    find_workspace_root().join("target")
}

/// Walk up from CARGO_MANIFEST_DIR until we find Cargo.toml with [workspace].
fn find_workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for p in manifest.ancestors() {
        let toml = p.join("Cargo.toml");
        if let Ok(s) = fs::read_to_string(&toml) {
            if s.contains("[workspace]") {
                return p.to_path_buf();
            }
        }
    }
    manifest
}

fn so_path() -> PathBuf {
    so_dir().join("libspike.so")
}

fn stamp_path() -> PathBuf {
    so_dir().join("libspike.stamp")
}

// ---------------------------------------------------------------------------
// Source hash
// ---------------------------------------------------------------------------

const SPIKE_LIBS: &[&str] = &["fesvr", "fdt", "softfloat", "disasm", "riscv"];

/// Compute a content-hash over all source files that affect the .so.
fn hash_sources() -> String {
    let mut hasher = Sha256::new();
    let spike_src = spike_src_dir();

    // Recursively hash source trees
    for sub in ["riscv", "softfloat", "fesvr", "fdt", "disasm"] {
        let dir = spike_src.join(sub);
        if dir.is_dir() {
            hash_dir(&mut hasher, &dir, &spike_src).ok();
        }
    }

    // Also hash wrapper source
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for f in ["src/wrapper.cc", "src/difftest_abi.h", "build.rs"] {
        let p = manifest.join(f);
        if p.is_file() {
            if let Ok(data) = fs::read(&p) {
                hasher.update(&data);
            }
        }
    }

    format!("{:x}", hasher.finalize())
}

fn hash_dir(hasher: &mut Sha256, dir: &Path, base: &Path) -> io::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for entry in entries {
        if entry.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        if entry.is_dir() {
            hash_dir(hasher, &entry, base)?;
        } else if entry.is_file() {
            hasher.update(&fs::read(&entry)?);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Build libspike.so
// ---------------------------------------------------------------------------

fn build_spike_so() -> Result<(), String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spike_src = spike_src_dir();
    let out = so_dir();
    fs::create_dir_all(&out).map_err(|e| format!("mkdir so dir: {e}"))?;

    let build_dir = out.join("spike-build");
    fs::create_dir_all(&build_dir).map_err(|e| format!("mkdir build dir: {e}"))?;

    let need_configure = !build_dir.join("Makefile").is_file();
    let steps = if need_configure { 4 } else { 3 };
    let spinner = new_spinner(steps);

    // --- configure (optional) ---
    if need_configure {
        let configure = spike_src.join("configure");
        if !configure.exists() {
            spinner.finish_with_message("spike configure not found");
            return Err(format!(
                "spike configure not found at {}",
                configure.display()
            ));
        }
        spinner.set_message("configuring spike...");
        let status = Command::new(&configure)
            .current_dir(&build_dir)
            .env("CFLAGS", "-O2 -fPIC")
            .env("CXXFLAGS", "-O2 -fPIC")
            .arg(format!("--srcdir={}", spike_src.display()))
            .args(["--with-boost-regex=no", "--with-boost-asio=no"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| format!("spike configure: {e}"))?;
        if !status.success() {
            spinner.finish_with_message("spike configure failed");
            return Err("spike configure failed".into());
        }
        spinner.inc(1);
    }

    // --- make ---
    spinner.set_message("building spike...");
    let make_status = Command::new("make")
        .current_dir(&build_dir)
        .env("CFLAGS", "-O2 -fPIC")
        .env("CXXFLAGS", "-O2 -fPIC")
        .arg("-j")
        .arg(num_cpus())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("spike make: {e}"))?;
    if !make_status.success() {
        spinner.finish_with_message("spike make failed");
        return Err("spike make failed".into());
    }
    spinner.inc(1);

    // --- verify static libs ---
    for lib in SPIKE_LIBS {
        let lib_path = build_dir.join(format!("lib{lib}.a"));
        if !lib_path.exists() {
            spinner.finish_with_message(format!("missing lib{lib}.a"));
            return Err(format!("missing lib{lib}.a"));
        }
    }

    // --- compile wrapper.cc ---
    spinner.set_message("compiling spike wrapper...");
    let wrapper_cc = manifest_dir.join("src/wrapper.cc");
    if !wrapper_cc.exists() {
        spinner.finish_with_message("wrapper.cc not found");
        return Err(format!("wrapper.cc not found at {}", wrapper_cc.display()));
    }
    let wrapper_o = out.join("spike_wrapper.o");
    let wrapper_status = Command::new("g++")
        .arg("-std=c++2a")
        .arg("-O2")
        .arg("-fPIC")
        .arg("-c")
        .arg(&wrapper_cc)
        .arg("-o")
        .arg(&wrapper_o)
        .arg("-I")
        .arg(manifest_dir.join("src"))
        .arg("-I")
        .arg(spike_src.join("riscv"))
        .arg("-I")
        .arg(spike_src.join("softfloat"))
        .arg("-I")
        .arg(spike_src.join("fesvr"))
        .arg("-I")
        .arg(&spike_src)
        .arg("-I")
        .arg(&build_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("compile wrapper.cc: {e}"))?;
    if !wrapper_status.success() {
        spinner.finish_with_message("wrapper.cc compile failed");
        return Err("compile wrapper.cc failed".into());
    }
    spinner.inc(1);

    // --- link everything into .so ---
    spinner.set_message("linking libspike.so...");
    let so = so_path();
    let so_tmp = so.with_extension("so.tmp");
    let mut cmd = Command::new("g++");
    cmd.arg("-shared").arg("-fPIC").arg("-o").arg(&so_tmp);
    cmd.arg(&wrapper_o);
    cmd.arg("-Wl,--whole-archive");
    for lib in SPIKE_LIBS {
        cmd.arg(build_dir.join(format!("lib{lib}.a")));
    }
    cmd.arg("-Wl,--no-whole-archive");
    cmd.arg("-lz").arg("-lpthread").arg("-ldl");
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    let link_status = cmd.status().map_err(|e| format!("g++ -shared: {e}"))?;
    if !link_status.success() {
        spinner.finish_with_message("linking failed");
        return Err("g++ -shared failed".into());
    }

    // Atomic rename
    fs::rename(&so_tmp, &so).map_err(|e| format!("rename .so: {e}"))?;

    // Write stamp
    let hash = hash_sources();
    fs::write(stamp_path(), &hash).map_err(|e| format!("write stamp: {e}"))?;

    spinner.finish_and_clear();
    Ok(())
}

// ---------------------------------------------------------------------------
// Load & global cache
// ---------------------------------------------------------------------------

static SPIKE_FNS: OnceLock<Result<SpikeFns, String>> = OnceLock::new();

/// Ensure libspike.so is built and loaded. Returns a reference to the
/// cached function table.
pub(crate) fn ensure_spike_loaded() -> Result<&'static SpikeFns, String> {
    SPIKE_FNS
        .get_or_init(|| {
            let so = so_path();
            let stamp = stamp_path();

            // Check if rebuild needed
            let fresh = if so.exists() && stamp.exists() {
                let stored = fs::read_to_string(&stamp).unwrap_or_default();
                let current = hash_sources();
                stored == current
            } else {
                false
            };

            if !fresh {
                if let Err(e) = build_spike_so() {
                    return Err(format!("spike .so build failed: {e}"));
                }
            }

            // Load
            let lib =
                unsafe { Library::new(&so).map_err(|e| format!("dlopen {}: {e}", so.display())) }?;

            SpikeFns::from_library(lib)
        })
        .as_ref()
        .map_err(|e| e.clone())
}

fn num_cpus() -> String {
    env::var("NUM_JOBS").unwrap_or_else(|_| {
        std::thread::available_parallelism()
            .map(|p| p.get().to_string())
            .unwrap_or_else(|_| "1".to_string())
    })
}

fn new_spinner(steps: u64) -> ProgressBar {
    let pb = ProgressBar::new(steps);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} [{pos}/{len}] {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

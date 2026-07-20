//! Lazy build & load of libnzea.so per (platform, isa) combination at runtime.
//!
//! Each combination has its own hash, build artifact directory, and .so.
//! The .so is built on first use and cached; subsequent calls with the same
//! (platform, isa) reuse the cached function table.
//!
//! Directory layout:
//!   target/remu-so/nzea/<platform>/<isa>/libnzea.so
//!   target/remu-so/nzea/<platform>/<isa>/libnzea.stamp
//!   target/remu-so/nzea/<platform>/<isa>/verilator_build/
//!   target/remu-so/nzea/<platform>/<isa>/verilog/

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use libloading::Library;
use nanospinner::Spinner;
use sha2::{Digest, Sha256};

use crate::NzeaTarget;
use crate::nzea_ffi::NzeaFns;

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

fn nzea_dir() -> PathBuf {
    let ws = find_workspace_root();
    env::var("NZEA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| ws.join("..").join("nzea"))
        .canonicalize()
        .unwrap_or_else(|_| ws.join("..").join("nzea"))
}

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

fn so_dir(target: &str, isa: &str) -> PathBuf {
    find_workspace_root()
        .join("target")
        .join("remu-so")
        .join("nzea")
        .join(target)
        .join(isa)
}

fn so_path(target: &str, isa: &str) -> PathBuf {
    so_dir(target, isa).join("libnzea.so")
}

fn stamp_path(target: &str, isa: &str) -> PathBuf {
    so_dir(target, isa).join("libnzea.stamp")
}

fn verilog_dir(target: &str, isa: &str) -> PathBuf {
    so_dir(target, isa).join(format!("nzea-verilog/{target}/{isa}"))
}

fn build_dir(target: &str, isa: &str) -> PathBuf {
    so_dir(target, isa).join("verilator_build")
}

fn wrapper_obj(target: &str, isa: &str) -> PathBuf {
    so_dir(target, isa).join("nzea_wrapper.o")
}

// ---------------------------------------------------------------------------
// Source hash (per combination)
// ---------------------------------------------------------------------------

fn hash_nzea_sources(target: &str, isa: &str) -> String {
    let mut hasher = Sha256::new();
    let nzea = nzea_dir();

    // Hash all .scala files
    hash_scala_files(&mut hasher, &nzea).ok();

    // Hash the justfile
    if let Ok(data) = fs::read(nzea.join("justfile")) {
        hasher.update(&data);
    }

    // Hash wrapper sources
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for f in ["c_src/nzea_wrapper.cpp"] {
        let p = manifest.join(f);
        if let Ok(data) = fs::read(&p) {
            hasher.update(&data);
        }
    }

    // Include target and isa in the hash (different ISAs produce different Verilog)
    hasher.update(target.as_bytes());
    hasher.update(isa.as_bytes());

    format!("{:x}", hasher.finalize())
}

fn hash_scala_files(hasher: &mut Sha256, dir: &Path) -> io::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for entry in entries {
        if entry
            .file_name()
            .is_some_and(|n| n == ".git" || n == "target")
        {
            continue;
        }
        if entry.is_dir() {
            hash_scala_files(hasher, &entry)?;
        } else if entry.extension().map_or(false, |e| e == "scala") {
            hasher.update(&fs::read(&entry)?);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Build libnzea.so for one (target, isa)
// ---------------------------------------------------------------------------

fn build_nzea_so(target: &NzeaTarget, isa_str: &str) -> Result<(), String> {
    let t = target.as_str();
    let top_module = target.top_module();
    let nzea = nzea_dir();
    let so_d = so_dir(t, isa_str);
    let vlog = verilog_dir(t, isa_str);
    let v_build = build_dir(t, isa_str);

    fs::create_dir_all(&so_d).map_err(|e| format!("mkdir: {e}"))?;

    let mut spinner = StepSpinner::new(4, format!("generating Verilog {t}:{isa_str}..."));

    // --- Step 1: just dump (Verilog generation) ---
    let justfile = nzea.join("justfile");
    if !justfile.exists() {
        spinner.fail(format!("justfile not found at {}", justfile.display()));
        return Err("build failed".into());
    }

    let mut dump = Command::new("direnv");
    dump.env("REQUIRE_FLAKE", "1")
        .arg("exec")
        .arg(&nzea)
        .arg("just")
        .arg("--justfile")
        .arg(&justfile)
        .arg("dump")
        .arg("--target")
        .arg(t)
        .arg("--isa")
        .arg(isa_str)
        .arg("--outDir")
        .arg(&vlog)
        .current_dir(&find_workspace_root());

    run_silent(&mut dump, &format!("just dump {t}:{isa_str}"))?;
    spinner.inc(format!("building model {t}:{isa_str}..."));

    // --- Step 2: verilator --cc --build ---
    fs::create_dir_all(&v_build).map_err(|e| format!("mkdir verilator_build: {e}"))?;

    let cc = env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let cxx = env::var("CXX").unwrap_or_else(|_| "g++".to_string());
    let ccache_cc = format!("ccache {cc}");
    let ccache_cxx = format!("ccache {cxx}");

    let verilog_rel = format!("nzea-verilog/{t}/{isa_str}");
    let filelist = vlog.join("filelist.f");
    let sv_files: Vec<String> = if filelist.exists() {
        let listed: Vec<String> = fs::read_to_string(&filelist)
            .unwrap_or_default()
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| s.ends_with(".sv"))
            .map(|s| format!("{verilog_rel}/{s}"))
            .collect();
        if !listed.is_empty() {
            listed
        } else {
            vec![format!("{verilog_rel}/{top_module}.sv")]
        }
    } else {
        vec![format!("{verilog_rel}/{top_module}.sv")]
    };

    // Append firtool memory init files
    let files = append_memory_inits(&vlog, &verilog_rel, sv_files);
    let files_arg = files.join(" ");
    let makeflags = format!("CC=\"{ccache_cc}\" CXX=\"{ccache_cxx}\"");
    let prefix = format!("VTop_{t}_{isa_str}");
    let cmd = format!(
        "cd '{}' && verilator --cc --build --trace-fst -MAKEFLAGS '{makeflags}' -CFLAGS -fPIC --Mdir {} --top-module {top_module} --prefix {prefix} {files_arg}",
        so_d.display(),
        v_build.display()
    );

    let mut vcmd = Command::new("sh");
    vcmd.args(["-c", &cmd])
        .env("CC", &ccache_cc)
        .env("CXX", &ccache_cxx);

    run_silent(&mut vcmd, &format!("verilator {t}:{isa_str}"))?;
    spinner.inc(format!("compiling wrapper {t}:{isa_str}..."));

    // --- Step 3: compile wrapper ---
    let wrapper_cc = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/nzea_wrapper.cpp");
    let wrapper_o = wrapper_obj(t, isa_str);
    let v_include = find_verilator_include()?;

    let mut wcmd = Command::new("g++");
    let prefix = format!("VTop_{t}_{isa_str}");
    wcmd.arg("-std=c++17")
        .arg("-O2")
        .arg("-fPIC")
        .arg("-c")
        .arg(&wrapper_cc)
        .arg("-o")
        .arg(&wrapper_o)
        .arg("-isystem")
        .arg(v_include.join("vltstd"))
        .arg("-isystem")
        .arg(&v_include)
        .arg("-I")
        .arg(&v_build)
        .arg("-Wno-unused-parameter")
        .arg("-Wno-sign-compare")
        .arg(format!("-DNZEA_MODEL_H=\"{prefix}.h\""))
        .arg(format!("-DNZEA_MODEL_TYPE={prefix}"))
        .arg(format!("-DNZEA_MODEL_KEY=\"{t}:{isa_str}\""));

    run_silent(&mut wcmd, &format!("wrapper compile {t}:{isa_str}"))?;
    spinner.inc(format!("linking {t}:{isa_str}..."));

    // --- Step 4: link .so ---
    let so = so_path(t, isa_str);
    let so_tmp = so.with_extension("so.tmp");
    let mut cmd = Command::new("g++");
    cmd.arg("-shared").arg("-fPIC").arg("-o").arg(&so_tmp);
    cmd.arg(&wrapper_o);
    cmd.arg("-Wl,--whole-archive");
    cmd.arg(v_build.join(format!("lib{prefix}.a")));
    cmd.arg(v_build.join("libverilated.a"));
    cmd.arg("-Wl,--no-whole-archive");
    cmd.arg("-lz");

    run_silent(&mut cmd, &format!("link {t}:{isa_str}"))?;
    fs::rename(&so_tmp, &so).map_err(|e| format!("rename: {e}"))?;

    // Write stamp
    let hash = hash_nzea_sources(t, isa_str);
    fs::write(stamp_path(t, isa_str), &hash).map_err(|e| format!("write stamp: {e}"))?;

    spinner.done();
    Ok(())
}

// ---------------------------------------------------------------------------
// Cache: one (target, isa) → NzeaFns
// ---------------------------------------------------------------------------

static NZEA_CACHE: Mutex<Option<HashMap<String, NzeaFns>>> = Mutex::new(None);

/// Ensure the nzea .so for the given combination is built and loaded.
pub(crate) fn ensure_nzea_loaded(target: &NzeaTarget, isa_str: &str) -> Result<(), String> {
    let key = format!("{}:{}", target.as_str(), isa_str);
    let mut cache = NZEA_CACHE.lock().map_err(|e| format!("lock: {e}"))?;
    if cache.is_none() {
        // DPI-C symbols are exported via --dynamic-list in remu_cli/build.rs.
        *cache = Some(HashMap::new());
    }
    let map = cache.as_mut().unwrap();

    if !map.contains_key(&key) {
        let t = target.as_str();
        let so = so_path(t, isa_str);
        let stamp = stamp_path(t, isa_str);

        let fresh = so.exists() && stamp.exists() && {
            let stored = fs::read_to_string(&stamp).unwrap_or_default();
            stored == hash_nzea_sources(t, isa_str)
        };

        if !fresh {
            build_nzea_so(target, isa_str)?;
        }

        let lib =
            unsafe { Library::new(&so).map_err(|e| format!("dlopen {}: {e}", so.display())) }?;
        let fns = NzeaFns::from_library(lib)?;
        map.insert(key.clone(), fns);
    }

    Ok(())
}

/// Get the cached function table for a combination.
/// Panics if `ensure_nzea_loaded` wasn't called first.
pub(crate) fn get_nzea_fns(target: &str, isa_str: &str) -> &'static NzeaFns {
    let key = format!("{}:{}", target, isa_str);
    let cache = NZEA_CACHE.lock().expect("nzea cache lock");
    let map = cache.as_ref().expect("nzea cache not initialized");
    // Safety: the HashMap is never freed (static Mutex), so references are 'static
    let ptr: *const NzeaFns = map.get(&key).expect("nzea not loaded for this combination");
    unsafe { &*ptr }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct StepSpinner {
    handle: nanospinner::SpinnerHandle,
    step: u64,
    total: u64,
}

impl StepSpinner {
    fn new(total: u64, msg: impl Into<String>) -> Self {
        let s = Spinner::new(&format!("[1/{total}] {}", msg.into()));
        Self {
            handle: s.start(),
            step: 1,
            total,
        }
    }
    fn inc(&mut self, msg: impl Into<String>) {
        self.step += 1;
        self.handle
            .update(format!("[{}/{}] {}", self.step, self.total, msg.into()));
    }
    fn done(&mut self) {
        self.handle.stop();
    }
    fn fail(&mut self, msg: impl Into<String>) {
        let _ = writeln!(io::stderr(), "{}", msg.into());
        self.handle.stop();
    }
}

fn append_memory_inits(verilog_dir: &Path, prefix: &str, mut files: Vec<String>) -> Vec<String> {
    let Ok(rd) = fs::read_dir(verilog_dir) else {
        return files;
    };
    let mut extra: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name();
            let name = name.to_str()?;
            if !name.ends_with("_init.sv") {
                return None;
            }
            Some(format!("{prefix}/{name}"))
        })
        .collect();
    extra.sort();
    files.extend(extra);
    files
}

fn find_verilator_include() -> Result<PathBuf, String> {
    if let Ok(root) = env::var("VERILATOR_ROOT") {
        let inc = PathBuf::from(&root).join("include");
        if inc.exists() {
            return Ok(inc);
        }
    }
    if let Ok(path_env) = env::var("PATH") {
        for p in env::split_paths(&path_env) {
            let exe = p.join("verilator");
            if exe.is_file() {
                if let Ok(exe_canon) = fs::canonicalize(&exe) {
                    if let Some(prefix) = exe_canon.parent().and_then(|p| p.parent()) {
                        for cand in [
                            prefix.join("share").join("verilator").join("include"),
                            prefix.join("include"),
                        ] {
                            if cand.exists() {
                                return Ok(cand);
                            }
                        }
                    }
                }
            }
        }
    }
    for cand in [
        PathBuf::from("/usr/share/verilator/include"),
        PathBuf::from("/usr/local/share/verilator/include"),
    ] {
        if cand.exists() {
            return Ok(cand);
        }
    }
    Err("verilator include not found".into())
}

/// Run a command silently; on failure, print its stderr and return an error.
fn run_silent(cmd: &mut Command, label: &str) -> Result<(), String> {
    let output = cmd.output().map_err(|e| format!("{label}: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr_tail = stderr
        .lines()
        .rev()
        .take(20)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    let _ = writeln!(
        io::stderr(),
        "\n--- {label} stderr (last 20 lines) ---\n{stderr_tail}"
    );
    Err(format!("{label} failed"))
}

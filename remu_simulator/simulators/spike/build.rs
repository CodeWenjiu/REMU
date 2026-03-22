use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const SPIKE_LIBS: &[&str] = &["fesvr", "fdt", "softfloat", "disasm", "riscv"];

fn main() {
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    // Debug: use -O1 for wrapper so glibc _FORTIFY_SOURCE does not warn (it requires -O).
    // -O1 compiles quickly; spike libs stay -O0 -g for fastest rebuild.
    let (spike_cflags, spike_cxxflags, wrapper_opt) = match profile.as_str() {
        "release" => ("-O3", "-O3", "3"),
        _ => ("-O0 -g", "-O0 -g", "1"),
    };

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let spike_src = manifest_dir.join("spike");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.join("spike-build");

    // Declare inputs early so any edit under spike sources reruns this build script.
    print_spike_rebuild_triggers(&manifest_dir);

    if !spike_src.join("configure").exists() {
        eprintln!(
            "cargo:warning=spike source not found at {}, skipping libspike build",
            spike_src.display()
        );
        return;
    }

    std::fs::create_dir_all(&build_dir).expect("create spike build dir");

    let makefile = build_dir.join("Makefile");
    if !makefile.is_file() {
        let configure = spike_src.join("configure");
        let configure_status = Command::new(&configure)
            .current_dir(&build_dir)
            .env("CFLAGS", spike_cflags)
            .env("CXXFLAGS", spike_cxxflags)
            .arg(format!("--srcdir={}", spike_src.display()))
            .args(["--with-boost-regex=no", "--with-boost-asio=no"])
            .arg(format!("--prefix={}", out_dir.display()))
            .status();

        let configure_ok = match configure_status {
            Ok(s) => s.success(),
            Err(e) => {
                eprintln!("cargo:warning=spike configure failed: {e}");
                return;
            }
        };

        if !configure_ok {
            eprintln!(
                "cargo:warning=spike configure failed. Need dtc, gcc, g++ etc. (nix develop or system-installed)"
            );
            return;
        }
    }

    let make_status = Command::new("make")
        .current_dir(&build_dir)
        .env("CFLAGS", spike_cflags)
        .env("CXXFLAGS", spike_cxxflags)
        .arg("-j")
        .arg(num_cpus())
        .status();

    let make_ok = match make_status {
        Ok(s) => s.success(),
        Err(e) => {
            eprintln!("cargo:warning=spike make failed: {e}");
            return;
        }
    };

    if !make_ok {
        eprintln!("cargo:warning=spike make failed");
        return;
    }

    for lib in SPIKE_LIBS {
        let lib_path = build_dir.join(format!("lib{lib}.a"));
        if !lib_path.exists() {
            eprintln!(
                "cargo:warning=expected lib{lib}.a not found at {}",
                lib_path.display()
            );
            return;
        }
    }

    // Compile wrapper.cc (needs spike headers and config.h)
    compile_wrapper(&manifest_dir, &spike_src, &build_dir, wrapper_opt);

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    for lib in SPIKE_LIBS {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-link-lib=dylib=dl");
    #[cfg(not(target_env = "msvc"))]
    {
        if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-lib=dylib=c++");
        } else {
            println!("cargo:rustc-link-lib=dylib=stdc++");
        }
    }
}

/// Any change under these trees (recursively) reruns build.rs → `make` relinks Spike libs / wrapper as needed.
fn print_spike_rebuild_triggers(manifest_dir: &Path) {
    println!("cargo:rerun-if-changed=build.rs");

    let rel_if_dir = |rel: &str| {
        let p = manifest_dir.join(rel);
        if p.is_dir() {
            println!("cargo:rerun-if-changed={rel}");
        }
    };

    // Spike autotools / top-level sources
    for f in ["spike/configure", "spike/configure.ac", "spike/Makefile.in"] {
        if manifest_dir.join(f).is_file() {
            println!("cargo:rerun-if-changed={f}");
        }
    }

    // Core C++ that ends up in static libs (insn .h/.cc live under riscv/)
    rel_if_dir("spike/riscv");
    rel_if_dir("spike/softfloat");
    rel_if_dir("spike/fesvr");
    rel_if_dir("spike/fdt");
    rel_if_dir("spike/disasm");
}

fn compile_wrapper(
    manifest_dir: &PathBuf,
    spike_src: &PathBuf,
    spike_build: &PathBuf,
    opt_level: &str,
) {
    let src_dir = manifest_dir.join("src");
    let wrapper_cc = src_dir.join("wrapper.cc");
    if !wrapper_cc.exists() {
        eprintln!(
            "cargo:warning=wrapper.cc not found at {}",
            wrapper_cc.display()
        );
        return;
    }

    let opt: u32 = opt_level.parse().unwrap_or(1);

    cc::Build::new()
        .cpp(true)
        .std("c++2a")
        .file(&wrapper_cc)
        .include(&src_dir)
        .include(spike_src.join("riscv"))
        .include(spike_src.join("softfloat"))
        .include(spike_src.join("fesvr"))
        .include(spike_src)
        .include(spike_build)
        .opt_level(opt)
        .compile("spike_wrapper");

    println!("cargo:rerun-if-changed=src/wrapper.cc");
    println!("cargo:rerun-if-changed=src/difftest_abi.h");
}

fn num_cpus() -> String {
    env::var("NUM_JOBS").unwrap_or_else(|_| {
        std::thread::available_parallelism()
            .map(|p| p.get().to_string())
            .unwrap_or_else(|_| "1".to_string())
    })
}

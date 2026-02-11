use std::env;
use std::path::PathBuf;
use std::process::Command;

const SPIKE_LIBS: &[&str] = &["fesvr", "fdt", "softfloat", "disasm", "riscv"];

fn main() {
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let (spike_cflags, spike_cxxflags, wrapper_opt) = match profile.as_str() {
        "release" => ("-O3", "-O3", "3"),
        _ => ("-O0 -g", "-O0 -g", "0"),
    };

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let spike_src = manifest_dir.join("spike");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.join("spike-build");

    if !spike_src.join("configure").exists() {
        eprintln!(
            "cargo:warning=spike source not found at {}, skipping libspike build",
            spike_src.display()
        );
        return;
    }

    std::fs::create_dir_all(&build_dir).expect("create spike build dir");

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

    println!("cargo:rerun-if-changed=spike/configure");
    println!("cargo:rerun-if-changed=spike/configure.ac");
}

fn compile_wrapper(manifest_dir: &PathBuf, spike_src: &PathBuf, spike_build: &PathBuf, opt_level: &str) {
    let src_dir = manifest_dir.join("src");
    let wrapper_cc = src_dir.join("wrapper.cc");
    let abi_h = src_dir.join("difftest_abi.h");

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
        .include(spike_src.join("fesvr"))
        .include(spike_src)
        .include(spike_build)
        .opt_level(opt)
        .compile("spike_wrapper");

    println!("cargo:rerun-if-changed={}", wrapper_cc.display());
    println!("cargo:rerun-if-changed={}", abi_h.display());
}

fn num_cpus() -> String {
    env::var("NUM_JOBS").unwrap_or_else(|_| {
        std::thread::available_parallelism()
            .map(|p| p.get().to_string())
            .unwrap_or_else(|_| "1".to_string())
    })
}

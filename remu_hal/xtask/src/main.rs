//! remu_hal tooling: build-app, run-remu (Cargo runner), and future tasks.

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, app, target] if cmd == "build-app" => build_app(app, target),
        [cmd, elf_path] if cmd == "run-remu" => run_remu(elf_path),
        _ => {
            eprintln!("Usage:");
            eprintln!("  xtask build-app <app> <target>");
            eprintln!("  xtask run-remu <elf-path>   # Cargo runner: load ELF and run on remu");
            eprintln!("  target: riscv32i, riscv32im, riscv32imac, or full triple");
            std::process::exit(1);
        }
    }
}

/// Cargo runner: load ELF into remu and run to exit. Infers ISA from path.
fn run_remu(elf_path: &str) {
    if !Path::new(elf_path).is_file() {
        eprintln!("run-remu: ELF file not found: {}", elf_path);
        std::process::exit(1);
    }

    // Infer ISA from path (e.g. .../riscv32im-unknown-none-elf/release/...)
    let isa = elf_path
        .split('/')
        .find(|s| s.ends_with("-unknown-none-elf"))
        .and_then(|s| s.strip_suffix("-unknown-none-elf"))
        .unwrap_or("riscv32i");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let status = Command::new("cargo")
        .current_dir(workspace_root)
        .args([
            "run",
            "-p",
            "remu_cli",
            "-q",
            "--release",
            "--",
            "--elf",
            elf_path,
            "--isa",
            isa,
            "--batch",
            "--startup",
            "continue",
        ])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("run-remu: failed to run remu_cli: {e}");
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(1));
}

fn expand_target(target: &str) -> String {
    if target.contains('-') {
        target.to_string()
    } else {
        format!("{target}-unknown-none-elf")
    }
}

fn build_app(app: &str, target: &str) {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    // xtask is at remu_hal/xtask, so parent = remu_hal, parent again = workspace root
    let hal_dir = Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .canonicalize()
        .expect("remu_hal root");
    let workspace_root = hal_dir.parent().unwrap();

    let triple = expand_target(target);
    let target_dir = workspace_root.join("target").join("app");
    let pkg = format!("remu_app_{app}");
    let manifest = workspace_root.join("Cargo.toml");
    let disasm_path = target_dir
        .join(&triple)
        .join("release")
        .join(format!("{pkg}.disasm"));

    let cargo = "cargo";
    let status = Command::new(cargo)
        .current_dir(&hal_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "build",
            "--release",
            "-p",
            &pkg,
            "--target",
            &triple,
            "-Z",
            "build-std=core",
            "--manifest-path",
            manifest.to_str().unwrap(),
        ])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Failed to run cargo: {e}");
            std::process::exit(1);
        });
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    let out = Command::new(cargo)
        .current_dir(&hal_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "objdump",
            "--release",
            "-p",
            &pkg,
            "--target",
            &triple,
            "--bin",
            &pkg,
            "-Z",
            "build-std=core",
            "--manifest-path",
            manifest.to_str().unwrap(),
            "--",
            "-d",
        ])
        .output()
        .expect("cargo objdump failed");
    if !out.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
        std::process::exit(1);
    }
    std::fs::write(&disasm_path, &out.stdout).expect("write disasm");
}

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
    let path = Path::new(elf_path);
    if !path.is_file() {
        eprintln!("run-remu: ELF file not found: {}", elf_path);
        std::process::exit(1);
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    // Generate disassembly: <elf>.asm
    if let Err(e) = gen_disasm(workspace_root, elf_path) {
        eprintln!("run-remu: disasm generation failed: {e}");
    }

    // Infer ISA from path (e.g. .../riscv32im-unknown-none-elf/release/...)
    let isa = elf_path
        .split('/')
        .find(|s| s.ends_with("-unknown-none-elf"))
        .and_then(|s| s.strip_suffix("-unknown-none-elf"))
        .unwrap_or("riscv32i");

    let batch = env::var("BATCH").is_ok();

    let mut args = vec![
        "run".into(),
        "-p".into(),
        "remu_cli".into(),
        "-q".into(),
        "--release".into(),
        "--".into(),
        "--elf".into(),
        elf_path.to_string(),
        "--isa".into(),
        isa.to_string(),
    ];

    if batch {
        args.push("--batch".into());
        args.push("--startup".into());
        args.push("continue".into());
    }
    if let Ok(v) = env::var("PLATFORM") {
        args.push("--platform".into());
        args.push(v);
    }
    if let Ok(v) = env::var("DIFFTEST") {
        args.push("--difftest".into());
        args.push(v);
    }

    let status = Command::new("cargo")
        .current_dir(workspace_root)
        .args(args)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("run-remu: failed to run remu_cli: {e}");
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(1));
}

/// Generate disassembly for elf_path, output to <elf_path>.asm.
fn gen_disasm(workspace_root: &Path, elf_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(elf_path);
    let pkg = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("invalid elf path")?;
    let triple = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("riscv32i-unknown-none-elf");

    let asm_path = path.with_extension("asm");
    let manifest = workspace_root.join("Cargo.toml");

    let out = Command::new("cargo")
        .current_dir(workspace_root)
        .args([
            "objdump",
            "--release",
            "-p",
            pkg,
            "--target",
            triple,
            "--bin",
            pkg,
            "-Z",
            "build-std=core,alloc",
            "--manifest-path",
            manifest.to_str().unwrap(),
            "--",
            "-d",
        ])
        .output()?;

    if out.status.success() {
        std::fs::write(&asm_path, &out.stdout)?;
    } else {
        return Err(String::from_utf8_lossy(&out.stderr).into());
    }
    Ok(())
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
    let asm_path = target_dir
        .join(&triple)
        .join("release")
        .join(format!("{pkg}.asm"));

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
            "build-std=core,alloc",
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
            "build-std=core,alloc",
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
    std::fs::write(&asm_path, &out.stdout).expect("write asm");
}

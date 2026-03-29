use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::{BuildAppArgs, PrintCmd, RunAppArgs, RunRemuArgs};
use crate::disasm::infer_isa_from_elf_path;
use crate::paths::Paths;
use crate::target::{
    artifact_dir_name, cargo_target_dir_subdir, merge_cargo_target_rv32im_rustflags,
    resolve_for_hal_dir, resolve_for_workspace_root, CARGO_TARGET_RUSTFLAGS_RV32IM_ENV,     EXISA0_ENV,
    WJ_CUS0_ISA_SUFFIX, REMU_ISA_ENV,
    ZVE32_REMU_ISA, ZVE32_TARGET_RUSTFLAGS,
};
use crate::util::shell_escape;

pub fn run(cmd: PrintCmd) -> ExitCode {
    match cmd {
        PrintCmd::RunApp(a) => print_run_app(a),
        PrintCmd::BuildApp(a) => print_build_app(a),
        PrintCmd::RunRemu(a) => print_run_remu(a),
    }
}

fn print_run_app(args: RunAppArgs) -> ExitCode {
    let paths = Paths::from_env();
    let ws = paths.workspace_canonical();
    let resolved = resolve_for_workspace_root(&ws, &args.target);
    let sub = cargo_target_dir_subdir(resolved.zve);
    let target_dir = ws.join("target").join(sub);
    let td = shell_escape(target_dir.to_str().expect("utf-8 path"));

    let pkg = format!("remu_app_{}", args.app);
    let tgt = shell_escape(&resolved.triple_or_json);

    let json = if resolved.needs_json_target_spec {
        " -Z json-target-spec"
    } else {
        ""
    };

    let body = if resolved.zve {
        format!(
            "export {isa_k}={isa_v}; export {rf_k}={rf_v}; cargo run -p {pkg} --target {tgt} --release -Z build-std=core,alloc{json}",
            isa_k = REMU_ISA_ENV,
            isa_v = shell_escape(ZVE32_REMU_ISA),
            rf_k = CARGO_TARGET_RUSTFLAGS_RV32IM_ENV,
            rf_v = shell_escape(&merge_cargo_target_rv32im_rustflags(ZVE32_TARGET_RUSTFLAGS)),
            pkg = shell_escape(&pkg),
            tgt = tgt,
            json = json,
        )
    } else {
        format!(
            "cargo run -p {pkg} --target {tgt} --release -Z build-std=core,alloc{json}",
            pkg = shell_escape(&pkg),
            tgt = tgt,
            json = json,
        )
    };

    println!(
        "(unset {REMU_ISA_ENV} {CARGO_TARGET_RUSTFLAGS_RV32IM_ENV}; export CARGO_TARGET_DIR={td}; {body})"
    );
    ExitCode::SUCCESS
}

fn print_build_app(args: BuildAppArgs) -> ExitCode {
    let paths = Paths::from_env();
    let hal_abs = paths.hal_canonical();
    let ws = paths.workspace_canonical();
    let resolved = resolve_for_hal_dir(&args.target);
    let sub = cargo_target_dir_subdir(resolved.zve);
    let target_dir = ws.join("target").join(sub);
    let artifact_dir = artifact_dir_name(&resolved.triple_or_json);
    let pkg = format!("remu_app_{}", args.app);
    let manifest = ws.join("Cargo.toml");

    let mut env_parts = vec![format!(
        "CARGO_TARGET_DIR={}",
        shell_escape(target_dir.to_str().expect("utf-8 path"))
    )];
    if resolved.zve {
        env_parts.push(format!(
            "{}={}",
            CARGO_TARGET_RUSTFLAGS_RV32IM_ENV,
            shell_escape(&merge_cargo_target_rv32im_rustflags(ZVE32_TARGET_RUSTFLAGS))
        ));
    }

    let json = if resolved.needs_json_target_spec {
        " -Z json-target-spec"
    } else {
        ""
    };

    let hal_s = shell_escape(hal_abs.to_str().expect("utf-8"));
    let manifest_s = shell_escape(manifest.to_str().expect("utf-8"));
    let pkg_s = shell_escape(&pkg);
    let triple_s = shell_escape(&resolved.triple_or_json);

    println!(
        "(cd {hal_s} && env {} cargo build --release -p {pkg_s} --target {triple_s} -Z build-std=core,alloc --manifest-path {manifest_s}{json})",
        env_parts.join(" ")
    );

    let elf = target_dir
        .join(&artifact_dir)
        .join("release")
        .join(&pkg);
    let elf_s = shell_escape(elf.to_str().expect("utf-8"));
    let asm = elf.with_extension("asm");
    let asm_s = shell_escape(asm.to_str().expect("utf-8"));
    println!("rust-objdump -d {elf_s} > {asm_s} || true");

    ExitCode::SUCCESS
}

fn print_run_remu(args: RunRemuArgs) -> ExitCode {
    let elf_path = &args.elf_path;
    if !elf_path.is_file() {
        eprintln!("print run-remu: ELF not found: {}", elf_path.display());
        return ExitCode::from(1);
    }

    let elf_abs: PathBuf = elf_path
        .canonicalize()
        .unwrap_or_else(|_| elf_path.to_path_buf());
    let elf_s = shell_escape(elf_abs.to_str().expect("utf-8"));
    let asm = elf_abs.with_extension("asm");
    let asm_s = shell_escape(asm.to_str().expect("utf-8"));

    println!("rust-objdump -d {elf_s} > {asm_s} || true");

    let mut isa: String = std::env::var(REMU_ISA_ENV)
        .unwrap_or_else(|_| infer_isa_from_elf_path(elf_abs.to_str().expect("utf-8")));
    if std::env::var(EXISA0_ENV).is_ok() && !isa.ends_with(WJ_CUS0_ISA_SUFFIX) {
        isa.push_str(WJ_CUS0_ISA_SUFFIX);
    }
    let isa_s = shell_escape(&isa);

    let paths = Paths::from_env();
    let manifest = paths.workspace_root.join("Cargo.toml");
    let manifest_s = shell_escape(manifest.to_str().expect("utf-8"));

    print!(
        "cargo run -p remu_cli --release --manifest-path {manifest_s} -- --elf {elf_s} --isa {isa_s}"
    );

    if std::env::var("BATCH").is_ok() {
        print!(" --batch --startup continue");
    }
    if let Ok(v) = std::env::var("PLATFORM") {
        print!(" --platform {}", shell_escape(&v));
    }
    if let Ok(v) = std::env::var("DIFFTEST") {
        print!(" --difftest {}", shell_escape(&v));
    }
    println!();

    ExitCode::SUCCESS
}

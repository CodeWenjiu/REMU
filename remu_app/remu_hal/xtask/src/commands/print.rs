use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::{BuildAppArgs, CheckAppArgs, PrintCmd, RunAppArgs, RunRemuArgs};
use crate::disasm::infer_isa_from_elf_path;
use crate::paths::Paths;
use crate::platform::PlatformConfig;
use crate::target::{
    artifact_dir_name, cargo_target_dir_subdir, merge_cargo_target_rustflags,
    remu_cli_cargo_release_suffix, resolve_for_hal_dir, resolve_for_workspace_root,
    CARGO_TARGET_RUSTFLAGS_RV32IM_ENV, CARGO_TARGET_RUSTFLAGS_RV32I_ENV, EXISA0_ENV, REMU_ISA_ENV,
    WJ_CUS0_ISA_SUFFIX, ZVE32_TARGET_RUSTFLAGS,
};
use crate::util::shell_escape;

pub(crate) fn run(cmd: PrintCmd) -> ExitCode {
    match cmd {
        PrintCmd::RunApp(a) => print_run_app(a),
        PrintCmd::BuildApp(a) => print_build_app(a),
        PrintCmd::RunRemu(a) => print_run_remu(a),
        PrintCmd::CheckApp(a) => print_check_app(a),
    }
}

// ── rcore platform (`just run-app … --platform rcore`) ───────────────────
//
// rcore is a *platform*, not an app: the M-mode firmware and S-mode kernel
// are injected by these recipes; the app itself is an ordinary `remu_app_*`
// package built as a U-mode program (its own linker.ld, base 0x8040_0000)
// and passed to remu_cli via `--app`.

/// True when the platform is rcore (firmware + kernel injected, app runs in
/// U mode under the kernel).
pub(crate) fn is_rcore_platform(p: &crate::platform::Platform) -> bool {
    *p == crate::platform::Platform::Rcore
}

/// rcore OS images: kernel + firmware, both built for the built-in
/// `riscv64im-unknown-none-elf` target; the user program uses the custom
/// `rcore64.json` target (`target_os = "rcore"`) so the U-mode app does not
/// drag in riscv-rt / remu_hal_embedded (M-mode runtime).
const RCORE_TARGET: &str = "riscv64im-unknown-none-elf";
const RCORE_CRATES: [&str; 2] = ["remu_firmware", "rcore_kernel"];
/// Custom target for U-mode user programs (see remu_app/rcore/rcore64.json).
const RCORE_USER_TARGET: &str = "rcore64";

/// Build the two rcore platform images (firmware + kernel).
fn print_rcore_builds(manifest_s: &str) {
    for pkg in RCORE_CRATES {
        println!(
            "cargo build -p {} --target {RCORE_TARGET} --release -Z build-std=core --features bare-metal --manifest-path {manifest_s}",
            shell_escape(pkg)
        );
    }
}

/// Path of the built user ELF for `app` (custom rcore target dir).
fn rcore_app_elf(ws: &std::path::Path, app: &str) -> String {
    let target_dir = ws.join("target").join(RCORE_USER_TARGET).join("release");
    let elf = target_dir.join(format!("remu_app_{}", app));
    shell_escape(elf.to_str().expect("utf-8"))
}

/// `just build-app APP riscv64im --platform rcore`: build firmware + kernel +
/// the user program (as a U-mode image).
fn print_build_rcore(app: &str) -> ExitCode {
    let paths = Paths::from_env();
    let ws = paths.workspace_canonical();
    let manifest = ws.join("Cargo.toml");
    let manifest_s = shell_escape(manifest.to_str().expect("utf-8"));
    print_rcore_builds(&manifest_s);
    // The user app builds for the custom rcore target (target_os = "rcore",
    // its own linker.ld via remu_hal_rcore, base 0x8040_0000). This avoids
    // riscv-rt entirely for U-mode apps; the `#[remu_hal::entry]` macro and
    // the HAL's rcore arm take care of the rest.
    let pkg = format!("remu_app_{}", app);
    let user_target = ws
        .join("remu_app/rcore")
        .join(format!("{RCORE_USER_TARGET}.json"));
    let user_target_s = shell_escape(user_target.to_str().expect("utf-8"));
    println!(
        "cargo build -p {} --target {user_target_s} --release -Z build-std=core,alloc -Z json-target-spec --manifest-path {manifest_s}",
        shell_escape(&pkg)
    );
    let elf = rcore_app_elf(&ws, app);
    println!("rust-objdump -d {elf} > {elf}.asm || true");
    ExitCode::SUCCESS
}

/// `just run-app APP riscv64im --platform rcore`: build firmware + kernel +
/// user app, then run remu_cli with `--firmware` + `--elf` + `--app` (the
/// rcore boot flow).
///
/// Bare run drops into the interactive REPL (no `--startup`), for debugging;
/// append `-- --batch --startup continue` for a non-interactive run.
fn print_run_rcore(args: &RunAppArgs) -> ExitCode {
    let paths = Paths::from_env();
    let ws = paths.workspace_canonical();
    let manifest = ws.join("Cargo.toml");
    let manifest_s = shell_escape(manifest.to_str().expect("utf-8"));

    print_rcore_builds(&manifest_s);
    let pkg = format!("remu_app_{}", args.app);
    let user_target = ws
        .join("remu_app/rcore")
        .join(format!("{RCORE_USER_TARGET}.json"));
    let user_target_s = shell_escape(user_target.to_str().expect("utf-8"));
    println!(
        "cargo build -p {} --target {user_target_s} --release -Z build-std=core,alloc -Z json-target-spec --manifest-path {manifest_s}",
        shell_escape(&pkg)
    );

    // The user program ELF lands under target/<rcore64>/release.
    let target_dir = ws.join("target").join(RCORE_USER_TARGET).join("release");
    let firmware = ws
        .join("target")
        .join(RCORE_TARGET)
        .join("release")
        .join("remu_firmware");
    let kernel = ws
        .join("target")
        .join(RCORE_TARGET)
        .join("release")
        .join("rcore_kernel");
    let firmware_s = shell_escape(firmware.to_str().expect("utf-8"));
    let kernel_s = shell_escape(kernel.to_str().expect("utf-8"));
    let app_s = shell_escape(
        target_dir
            .join(format!("remu_app_{}", args.app))
            .to_str()
            .expect("utf-8"),
    );

    let rel = remu_cli_cargo_release_suffix();
    print!(
        "cargo run -p remu_cli{rel} --manifest-path {manifest_s} -- --firmware {firmware_s} --elf {kernel_s} --app {app_s} --isa riscv64im --platform remu"
    );
    // `just` forwards the user's `--` separator verbatim into `remu_cli_args`;
    // it is meaningless to remu_cli (each arg is already shell-delimited), so
    // drop it before forwarding.
    for arg in args.remu_cli_args.iter().filter(|a| a.as_str() != "--") {
        print!(" {}", shell_escape(arg));
    }
    println!();

    ExitCode::SUCCESS
}

/// Validation-only subcommand (prints nothing on success): usable from scripts
/// that build without xtask, e.g. `just run-app --platform host`.
fn print_check_app(args: CheckAppArgs) -> ExitCode {
    let paths = Paths::from_env();
    let ws = paths.workspace_canonical();
    match crate::app_caps::validate_app_target(&ws, &args.app, &args.target) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::from(1)
        }
    }
}

/// rcore runs the app in U mode under a `riscv64im` kernel+firmware; the
/// user app itself must be a `remu_app_*` package and target riscv64im.
fn check_rcore_platform(args: &RunAppArgs) -> Result<(), String> {
    if args.target != RCORE_TARGET && args.target != "riscv64im" {
        return Err(format!(
            "xtask: rcore platform requires target `riscv64im` (got `{}`): \
             the kernel/firmware are built for {RCORE_TARGET}",
            args.target
        ));
    }
    let paths = Paths::from_env();
    let ws = paths.workspace_canonical();
    crate::app_caps::validate_app_target(&ws, &args.app, "riscv64im")
}

fn print_run_app(args: RunAppArgs) -> ExitCode {
    // rcore platform: firmware + kernel injected, app runs in U mode.
    if is_rcore_platform(&args.platform) {
        if let Err(e) = check_rcore_platform(&args) {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
        return print_run_rcore(&args);
    }
    let paths = Paths::from_env();
    let ws = paths.workspace_canonical();
    let resolved = match resolve_for_workspace_root(&ws, &args.target) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("xtask: {e}");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = crate::app_caps::validate_app_target(&ws, &args.app, &args.target) {
        eprintln!("xtask: {e}");
        return ExitCode::from(1);
    }
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

    let mut exports: Vec<String> = Vec::new();

    // Platform-specific linker flags (from PlatformConfig trait). When the
    // platform RUSTFLAGS env key collides with the zve one (same target triple),
    // fold both into a single export — two exports of one variable would let the
    // second clobber the first (losing `-Tmemory.x` and the zve feature flags).
    let pf = args.platform.rustflags();
    if !pf.is_empty() {
        let env_key = format!(
            "CARGO_TARGET_{}_RUSTFLAGS",
            resolved.triple_or_json.to_uppercase().replace('-', "_")
        );
        if resolved.zve_cargo_rustflags_env == Some(env_key.as_str()) {
            let combined = format!("{} {}", pf.join(" "), ZVE32_TARGET_RUSTFLAGS);
            let rf_v = shell_escape(&merge_cargo_target_rustflags(&env_key, &combined));
            exports.push(format!("{env_key}={rf_v}"));
        } else {
            exports.push(format!("{env_key}={}", shell_escape(&pf.join(" "))));
        }
    }

    if let Some(env_k) = resolved.zve_cargo_rustflags_env {
        if !exports.iter().any(|e| e.starts_with(&format!("{env_k}="))) {
            let rf_v = shell_escape(&merge_cargo_target_rustflags(env_k, ZVE32_TARGET_RUSTFLAGS));
            exports.push(format!("{env_k}={rf_v}"));
        }
    }
    if let Some(isa) = &resolved.remu_isa {
        exports.push(format!("{}={}", REMU_ISA_ENV, shell_escape(isa)));
    }
    let export_prefix = if exports.is_empty() {
        String::new()
    } else {
        format!("export {}; ", exports.join("; export "))
    };

    let remu_cli_args = args
        .remu_cli_args
        .iter()
        .map(|s| shell_escape(s))
        .collect::<Vec<_>>();
    let forward_args = if remu_cli_args.is_empty() {
        String::new()
    } else {
        format!(" -- {}", remu_cli_args.join(" "))
    };

    let body = format!(
        "{export_prefix}cargo run -p {pkg} --target {tgt} --release -Z build-std=core,alloc{json}{forward_args}",
        pkg = shell_escape(&pkg),
        tgt = tgt,
        json = json,
        forward_args = forward_args,
    );

    println!(
        "(unset {REMU_ISA_ENV} {CARGO_TARGET_RUSTFLAGS_RV32I_ENV} {CARGO_TARGET_RUSTFLAGS_RV32IM_ENV}; export CARGO_TARGET_DIR={td}; {body})"
    );
    ExitCode::SUCCESS
}

fn print_build_app(args: BuildAppArgs) -> ExitCode {
    // rcore platform: firmware + kernel + user app (U mode) all built.
    if is_rcore_platform(&args.platform) {
        if let Err(e) = check_rcore_platform(&RunAppArgs {
            app: args.app.clone(),
            target: args.target.clone(),
            platform: args.platform,
            remu_cli_args: vec![],
        }) {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
        return print_build_rcore(&args.app);
    }
    let paths = Paths::from_env();
    let hal_abs = paths.hal_canonical();
    let ws = paths.workspace_canonical();
    let resolved = match resolve_for_hal_dir(&args.target) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("xtask: {e}");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = crate::app_caps::validate_app_target(&ws, &args.app, &args.target) {
        eprintln!("xtask: {e}");
        return ExitCode::from(1);
    }
    let sub = cargo_target_dir_subdir(resolved.zve);
    let target_dir = ws.join("target").join(sub);
    let artifact_dir = artifact_dir_name(&resolved.triple_or_json);
    let pkg = format!("remu_app_{}", args.app);
    let manifest = ws.join("Cargo.toml");

    let mut env_parts = vec![format!(
        "CARGO_TARGET_DIR={}",
        shell_escape(target_dir.to_str().expect("utf-8 path"))
    )];
    if let Some(env_k) = resolved.zve_cargo_rustflags_env {
        env_parts.push(format!(
            "{}={}",
            env_k,
            shell_escape(&merge_cargo_target_rustflags(env_k, ZVE32_TARGET_RUSTFLAGS,))
        ));
    }

    let json = if resolved.needs_json_target_spec {
        " -Z json-target-spec"
    } else {
        ""
    };

    let rustflags_parts: Vec<String> = args.platform.rustflags();
    if !rustflags_parts.is_empty() {
        let env_key = format!(
            "CARGO_TARGET_{}_RUSTFLAGS",
            resolved.triple_or_json.to_uppercase().replace('-', "_")
        );
        env_parts.push(format!(
            "{}={}",
            env_key,
            shell_escape(&rustflags_parts.join(" "))
        ));
    }

    let hal_s = shell_escape(hal_abs.to_str().expect("utf-8"));
    let manifest_s = shell_escape(manifest.to_str().expect("utf-8"));
    let pkg_s = shell_escape(&pkg);
    let triple_s = shell_escape(&resolved.triple_or_json);

    println!(
        "(cd {hal_s} && env {} cargo build --release -p {pkg_s} --target {triple_s} -Z build-std=core,alloc --manifest-path {manifest_s}{json})",
        env_parts.join(" ")
    );

    let elf = target_dir.join(&artifact_dir).join("release").join(&pkg);
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

    let rel = remu_cli_cargo_release_suffix();
    print!(
        "cargo run -p remu_cli{rel} --manifest-path {manifest_s} -- --elf {elf_s} --isa {isa_s}"
    );

    for arg in args.remu_cli_args {
        print!(" {}", shell_escape(&arg));
    }
    println!();

    ExitCode::SUCCESS
}

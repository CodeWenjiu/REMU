//! FPGA board deploy: hand a built ELF to the board's `probe` tool.
//!
//! The board's boot loader takes an ELF directly — `probe load` sends the
//! `PT_LOAD` file bytes as one loader frame — so remu owns no image-format
//! knowledge here: it drives the same nzea `just` entry point the RTL dump
//! uses, and the tool does the rest. Deploying builds no simulator and needs
//! neither state nor an ISA.

use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;

use crate::runtime;

/// What an nzea invocation asks for, from `--sim-opt action=`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NzeaAction {
    /// Simulate the RTL (the default).
    #[default]
    Run,
    /// Deploy the program to the FPGA board's boot loader.
    Flash,
}

impl FromStr for NzeaAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "run" => Ok(Self::Run),
            "flash" => Ok(Self::Flash),
            other => Err(format!(
                "invalid nzea action {other:?} (expected run or flash)"
            )),
        }
    }
}

/// Send `elf` to the board's boot loader (`just probe load <elf>`). The tool's
/// own output — the load's read-back of the program's UART — goes to our stdio.
pub fn flash(elf: &Path) -> Result<(), String> {
    if !elf.is_file() {
        return Err(format!("ELF to flash not found: {}", elf.display()));
    }
    let mut cmd = runtime::nzea_just(&[OsStr::new("probe"), OsStr::new("load"), elf.as_os_str()]);
    let status = cmd
        .status()
        .map_err(|e| format!("could not run the nzea probe: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("probe load failed ({status})"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_parses_run_and_flash_only() {
        assert_eq!("run".parse::<NzeaAction>(), Ok(NzeaAction::Run));
        assert_eq!("flash".parse::<NzeaAction>(), Ok(NzeaAction::Flash));
        assert_eq!(NzeaAction::default(), NzeaAction::Run);
        assert!("frobnicate".parse::<NzeaAction>().is_err());
    }

    #[test]
    fn probe_load_is_invoked_through_the_nzea_justfile() {
        let cmd = runtime::nzea_just(&[
            OsStr::new("probe"),
            OsStr::new("load"),
            OsStr::new("app.elf"),
        ]);
        assert_eq!(cmd.get_program(), "direnv");
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "exec");
        assert!(args[2] == "just" && args[3] == "--justfile", "{args:?}");
        assert!(args[4].ends_with("justfile"), "{args:?}");
        assert_eq!(args.len(), 8);
        assert_eq!(&args[5], "probe");
        assert_eq!(&args[6], "load");
        assert_eq!(&args[7], "app.elf");
    }

    #[test]
    fn a_missing_elf_is_refused_before_probing() {
        let err = flash(Path::new("/nonexistent/remu-flash-test.elf")).unwrap_err();
        assert!(err.contains("not found"), "{err}");
    }
}

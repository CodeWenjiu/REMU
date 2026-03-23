{
  description = "Flake configuration for remu";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      nixpkgs,
      utils,
      rust-overlay,
      ...
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        # Runtime dlopen (GPUI / winit / Wayland): mkShell alone does not always put these on LD_LIBRARY_PATH.
        guiRuntime = with pkgs; [
          wayland
          libxkbcommon
          libGL
          vulkan-loader
          libX11
          libXcursor
          libXrandr
          libXi
          libxcb
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          TMPDIR = "/tmp";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

          buildInputs =
            (with pkgs; [
            # rust toolchain (nightly for error_generic_member_access / #[backtrace])
            (rust-bin.nightly.latest.default.override {
              extensions = [
                "rust-src"
                "clippy"
                "rust-analyzer"
                "llvm-tools-preview"
              ];
              targets = [
                "riscv32i-unknown-none-elf"
                "riscv32im-unknown-none-elf"
                "riscv32imac-unknown-none-elf"
              ];
            })
            cargo-machete
            cargo-show-asm
            cargo-binutils
            cargo-edit

            gawk
            just

            clang
            libclang
            cmake
            verilator
            ccache

            mold

            gnumake
            gcc
            autoconf
            automake
            libtool
            pkg-config
            dtc
            bison
            flex
            python3
            zlib

            gource
            ])
            ++ guiRuntime;

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath guiRuntime}''${LD_LIBRARY_PATH:+:}$LD_LIBRARY_PATH"
            export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPath "lib/pkgconfig" (builtins.map pkgs.lib.getDev guiRuntime)}''${PKG_CONFIG_PATH:+:}$PKG_CONFIG_PATH"
          '';
        };
      }
    );
}

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

        rust-toolchain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [
            "rust-src"
            "clippy"
            "rust-analyzer"
            "llvm-tools-preview"
            "rustc-codegen-cranelift-preview"
          ];
          targets = [
            "riscv32i-unknown-none-elf"
            "riscv32im-unknown-none-elf"
            "riscv32imac-unknown-none-elf"
          ];
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

        devPackages = [
          rust-toolchain
          pkgs.stdenv.cc
        ]
        ++ guiRuntime
        ++ (with pkgs; [
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
          qemu
          spike
          ccache
          sccache

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
        ]);
      in
      {
        devShells.default = pkgs.mkShell {
          TMPDIR = "/tmp";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

          buildInputs = devPackages;

          shellHook = ''
            export RUSTC_WRAPPER=sccache
            export SCCACHE_DIR="$PWD/.sccache"
            mkdir -p .direnv/bin
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath guiRuntime}:$LD_LIBRARY_PATH
            export LD_LIBRARY_PATH=${pkgs.zlib.out}/lib:$LD_LIBRARY_PATH
            export LIBRARY_PATH=${pkgs.zlib.out}/lib
            export OPENSSL_NO_VENDOR=1
            export OPENSSL_DIR=${pkgs.openssl.dev}
            export OPENSSL_LIB_DIR=${pkgs.openssl.out}/lib
            export OPENSSL_INCLUDE_DIR=${pkgs.openssl.dev}/include
            export PKG_CONFIG_PATH=${pkgs.openssl.dev}/lib/pkgconfig
          ''
          + pkgs.lib.concatMapStringsSep "\n" (pkg: ''
            if [ -d "${pkg}/bin" ]; then
              for f in ${pkg}/bin/*; do
                ln -sf "$f" .direnv/bin/
              done
            fi
          '') devPackages;
        };
      }
    );
}

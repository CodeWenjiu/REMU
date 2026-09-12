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

        # Build `spike` from the vendored submodule (riscv-isa-sim 1.1.1-dev)
        # instead of the much older nixpkgs `spike` (1.1.0, 2021).
        # `builtins.fetchGit` snapshots the submodule's git repo directly from
        # the local checkout (no re-clone, no network). Note: it must use an
        # absolute path — within a flake, `./…` resolves to the flake's own
        # store copy, which never contains submodule content.
        spikeSrc = builtins.fetchGit {
          url = "/home/wenjiu/project/chip-dev/remu/remu_simulator/simulators/spike/spike";
          rev = "c09c0cce98696f52abe0fe8c11f93f9ed74dc2bb";
        };

        spike = pkgs.stdenv.mkDerivation {
          pname = "spike";
          version = "1.1.1-dev";
          src = spikeSrc;
          nativeBuildInputs = [
            pkgs.autoconf
            pkgs.automake
            pkgs.libtool
            pkgs.pkg-config
            pkgs.dtc
          ];
          buildInputs = [ pkgs.zlib ];
          preConfigure = ''
            autoreconf -i
          '';
          configureFlags = [ "--with-boost=no" ];
          meta = {
            description = "RISC-V ISA Simulator (from the remu spike submodule)";
            homepage = "https://github.com/riscv-software-src/riscv-isa-sim";
            license = pkgs.lib.licenses.bsd3;
          };
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
          ccache
          sccache

          # Performance profiling toolchain
          perf
          hyperfine
          valgrind

          # Wayland virtual-keyboard input simulation (test-only)
          wtype

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
          lz4

          gource
        ])
        ++ [
          spike
        ];
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
            export LD_LIBRARY_PATH=${pkgs.zlib.out}/lib:${pkgs.lz4.lib}/lib:$LD_LIBRARY_PATH
            export LIBRARY_PATH=${pkgs.zlib.out}/lib:${pkgs.lz4.lib}/lib
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

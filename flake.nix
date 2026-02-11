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
      in
      {
        devShells.default = pkgs.mkShell {
          TMPDIR = "/tmp";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

          buildInputs = with pkgs; [
            # rust toolchain (nightly for error_generic_member_access / #[backtrace])
            (rust-bin.nightly.latest.default.override {
              extensions = [
                "rust-src"
                "clippy"
                "rust-analyzer"
                "llvm-tools-preview"
              ];
            })
            cargo-edit
            cargo-machete
            cargo-show-asm

            gawk

            clang
            libclang
            cmake

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
          ];
        };
      }
    );
}

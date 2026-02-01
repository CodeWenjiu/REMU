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
          # 避免 Nix 使用不存在的 /tmp/nix-shell.xxx 导致 rustc/rust-analyzer proc-macro 退出 101
          TMPDIR = "/tmp";

          buildInputs = with pkgs; [
            # rust toolchain
            (rust-bin.stable.latest.default.override {
              extensions = [
                "rust-src"
                "clippy"
                "rust-analyzer"
                "llvm-tools-preview"
              ];
            })
            cargo-edit
            cargo-machete

            clang
            mold
          ];

          shellHook = ''
            # 确保 TMPDIR 指向存在的目录，避免 rustc/rust-analyzer proc-macro 因无法创建临时目录而退出 101
            if [ -z "$TMPDIR" ] || [ ! -d "$TMPDIR" ]; then
              export TMPDIR=/tmp
            fi

            if [ -z "''${_NU_LAUNCHED:-}" ]; then
              export _NU_LAUNCHED=1
              nu
            fi
          '';
        };
      }
    );
}

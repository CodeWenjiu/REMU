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
        # Everything the dev shell is made of lives in nix/.
        devShells.default = import ./nix/devshell.nix {
          inherit pkgs;
          rust = import ./nix/rust.nix { inherit pkgs; };
          spike = import ./nix/spike.nix { inherit pkgs; };
          gui = import ./nix/gui.nix { inherit pkgs; };
          mujoco = import ./nix/mujoco.nix { inherit pkgs; };
        };
      }
    );
}
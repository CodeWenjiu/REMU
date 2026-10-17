# Build `spike` from the vendored submodule (riscv-isa-sim 1.1.1-dev)
# instead of the much older nixpkgs `spike` (1.1.0, 2021).
# `builtins.fetchGit` snapshots the submodule's git repo directly from
# the local checkout (no re-clone, no network). Note: it must use an
# absolute path — within a flake, `./…` resolves to the flake's own
# store copy, which never contains submodule content.
{ pkgs }:

let
  spikeSrc = fetchGit {
    url = "/home/wenjiu/project/chip-dev/remu/remu_simulator/simulators/spike/spike";
    rev = "c09c0cce98696f52abe0fe8c11f93f9ed74dc2bb";
  };
in
pkgs.stdenv.mkDerivation {
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
}
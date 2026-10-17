# The remu dev shell: assembles the toolchains, tools, and environment
# exports from the sibling files (rust / spike / gui / mujoco).
{ pkgs, rust, spike, gui, mujoco }:

let
  devPackages = [ rust pkgs.stdenv.cc ] ++ gui ++ (with pkgs; [
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
    # Slint's host font stack (fontique -> yeslogic-fontconfig-sys) links
    # libfontconfig; the shell needs its pkg-config file to build.
    fontconfig

    gource
  ]) ++ [
    mujoco.engine
    spike
  ];
in
pkgs.mkShell {
  TMPDIR = "/tmp";
  LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

  buildInputs = devPackages;

  shellHook = ''
    export RUSTC_WRAPPER=sccache
    export SCCACHE_DIR="$PWD/.sccache"
    mkdir -p .direnv/bin
    export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath gui}:$LD_LIBRARY_PATH
    export LD_LIBRARY_PATH=${pkgs.zlib.out}/lib:${pkgs.lz4.lib}/lib:$LD_LIBRARY_PATH
    export LIBRARY_PATH=${pkgs.zlib.out}/lib:${pkgs.lz4.lib}/lib
    ${mujoco.shellHook}
    # glvnd must find this shell's own Mesa: the system drivers under
    # /run/opengl-driver are built against a newer glibc than the
    # shell's, so loading them fails.
    export __EGL_VENDOR_LIBRARY_DIRS=${pkgs.mesa}/share/glvnd/egl_vendor.d
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
}
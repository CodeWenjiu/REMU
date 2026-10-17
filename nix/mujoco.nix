# MuJoCo engine + model assets for the remu dev shell.
#
# The engine is dlopen'd at runtime (never linked); the shell exports
# MUJOCO_DIR so the loader finds libmujoco.so. The Menagerie fetch is pinned
# and sparse: only the Franka Emika Panda directory lands in the store, and
# MUJOCO_MENAGERIE_DIR points the arm device (`REMU_ARM_MODEL`) at it.
{ pkgs }:

let
  engine = pkgs.mujoco;

  menagerie = pkgs.fetchgit {
    url = "https://github.com/google-deepmind/mujoco_menagerie";
    rev = "62ab00f2bc44f6f20c1c9b7a13f3fb1dd41a4e35";
    sparseCheckout = [ "franka_emika_panda" ];
    hash = "sha256-vmug21NrlzDTCD2yTUeCkNMOPIvSZ74nBFSCRJ7HNVc=";
  };
in
{
  inherit engine menagerie;

  # Shell-hook fragment: the environment the runtime loaders and demos rely on.
  shellHook = ''
    export MUJOCO_DIR=${engine}
    export LD_LIBRARY_PATH=${engine}/lib:$LD_LIBRARY_PATH
    export MUJOCO_MENAGERIE_DIR=${menagerie}
  '';
}
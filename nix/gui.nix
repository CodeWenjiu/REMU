# GUI runtime libraries for dlopen (GPUI / winit / Wayland): mkShell alone
# does not always put these on LD_LIBRARY_PATH.
{ pkgs }:

with pkgs; [
  wayland
  libxkbcommon
  libGL
  mesa
  vulkan-loader
  libX11
  libXcursor
  libXrandr
  libXi
  libxcb
]
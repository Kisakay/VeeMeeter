# Libraries loaded with dlopen at runtime by winit/glutin (no link-time dependency).
pkgs: with pkgs; [
  libGL
  libxkbcommon
  wayland
  libx11
  libxcursor
  libxi
  libxrandr
]

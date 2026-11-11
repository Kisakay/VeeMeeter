#!/usr/bin/env bash
# Regenerate every brand SVG, check the palette, export the PNGs.
# Usage: branding/src/build.sh   (needs python3; PNG export uses nix for resvg + Nunito)
set -euo pipefail
cd "$(dirname "$0")"
python3 mascot.py
python3 logo.py
python3 decor.py
python3 palette.py
if command -v nix >/dev/null; then
  resvg=$(nix build --no-link --print-out-paths nixpkgs#resvg)/bin/resvg
  fonts=$(nix build --no-link --print-out-paths nixpkgs#nunito)/share/fonts
  for f in ../social/banner ../social/ui-mockup; do
    "$resvg" --skip-system-fonts --use-fonts-dir "$fonts" "$f.svg" "$f.png"
  done
  for s in 16 32 48 256 512; do
    src=../logo/mark.svg; [ "$s" -le 48 ] && src=../logo/mark-small.svg
    "$resvg" -w "$s" -h "$s" "$src" "../logo/png/veemeeter-$s.png"
  done
  # Mimi dans la fenêtre de l'application (src/gui/mascot.rs).
  for m in wave happy sleepy surprised; do
    "$resvg" -w 192 -h 192 "../mascot/mimi-$m.svg" "../../assets/gui/mimi-$m.png"
  done
fi

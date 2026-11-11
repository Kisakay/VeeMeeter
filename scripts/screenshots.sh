#!/usr/bin/env bash
# Régénère les captures du README (docs/assets/screenshots/) : la vraie GUI,
# dans un serveur X virtuel, branchée sur le faux daemon de démonstration.
# Aucune donnée de la session réelle n'apparaît.
#   usage : scripts/screenshots.sh   (depuis la racine, après `cargo build`)
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=docs/assets/screenshots
mkdir -p "$OUT"
tool() { echo "$(nix build --no-link --print-out-paths "nixpkgs#$1")/bin"; }
XVFB=$(tool xvfb-run) IM=$(tool imagemagick) XDO=$(tool xdotool)
# shellcheck disable=SC2016  # expansion voulue dans le shell nix develop
LIBS=$(nix develop -c bash -c 'echo $LD_LIBRARY_PATH' 2>/dev/null)
RT=$(mktemp -d)
trap 'kill $(jobs -p) 2>/dev/null; rm -rf "$RT"' EXIT

shoot() { # shoot <fichier> <démon:0|1> [clic x y]
  local file=$1 daemon=$2 click=${3:-}
  rm -f "$RT/veemeeter.sock"
  if [ "$daemon" = 1 ]; then XDG_RUNTIME_DIR=$RT python3 scripts/demo-daemon.py & sleep 0.5; fi
  "$XVFB/xvfb-run" -a -s "-screen 0 1420x740x24" bash -c "
    unset WAYLAND_DISPLAY
    export XDG_RUNTIME_DIR=$RT LD_LIBRARY_PATH=$LIBS:/run/opengl-driver/lib
    ./target/debug/veemeeter gui --no-spawn >/dev/null 2>&1 &
    sleep 6
    if [ -n '$click' ]; then $XDO/xdotool mousemove $click click 1; sleep 1.5; fi
    $IM/import -window root '$OUT/$file'
    kill %1"
  [ "$daemon" = 1 ] && kill %1 2>/dev/null || true
  wait 2>/dev/null || true
  echo "$OUT/$file"
}

shoot mixer.png 1
shoot strip-menu.png 1 "481 107"
shoot fx-menu.png 1 "114 118"
shoot waking.png 0

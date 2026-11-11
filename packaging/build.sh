#!/usr/bin/env bash
# Construit, teste et empaquette VeeMeeter DANS un conteneur de distribution.
#   usage : packaging/build.sh <debian|ubuntu|fedora|arch>   (depuis la racine)
# Utilisé par la CI (.github/workflows/ci.yml) et en local :
#   podman run --rm --cpus 4 -v "$PWD":/src:Z -w /src debian:trixie packaging/build.sh debian
# Les paquets atterrissent dans dist/.
set -euo pipefail
family=${1:?usage: packaging/build.sh <debian|ubuntu|fedora|arch>}
jobs=${CARGO_BUILD_JOBS:-4}
export CARGO_BUILD_JOBS=$jobs CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target/$family}

case $family in
  debian|ubuntu)
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq
    apt-get install -y -qq --no-install-recommends \
      ca-certificates curl build-essential pkg-config clang libclang-dev libpipewire-0.3-dev >/dev/null
    ;;
  fedora)
    dnf install -y -q gcc clang clang-devel pkgconf-pkg-config pipewire-devel curl >/dev/null
    ;;
  arch)
    pacman -Syu --noconfirm --needed base-devel clang pipewire curl >/dev/null
    ;;
  *) echo "unknown family $family" >&2; exit 2 ;;
esac

# Rust stable via rustup (les rustc des distros LTS sont trop anciens pour l'édition 2024).
if ! command -v cargo >/dev/null; then
  curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal --component clippy
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi

cargo build --release --locked
cargo test --release --locked
mkdir -p dist
# Sans tiret : interdit dans une release RPM, ambigu dans une révision Debian.
# shellcheck disable=SC1091
id=$( . /etc/os-release && echo "${ID}${VERSION_ID:-}" )
case $family in
  debian|ubuntu)
    cargo install --locked -q cargo-deb
    # cargo-deb trouve le binaire via CARGO_TARGET_DIR (exporté plus haut).
    cargo deb --no-build --no-strip -o "dist/" --deb-revision "1~$id"
    ;;
  fedora)
    cargo install --locked -q cargo-generate-rpm
    cargo generate-rpm --target-dir "$CARGO_TARGET_DIR" -o "dist/" -s "release = '1.$id'"
    ;;
  arch)
    # Arborescence à extraire dans / : `sudo tar -xzf veemeeter-*.tar.gz -C /`.
    root=dist/root
    rm -rf "$root"
    install -Dm755 "$CARGO_TARGET_DIR/release/veemeeter" "$root/usr/bin/veemeeter"
    install -Dm644 assets/veemeeter.desktop "$root/usr/share/applications/veemeeter.desktop"
    install -Dm644 assets/veemeeter.svg "$root/usr/share/icons/hicolor/scalable/apps/veemeeter.svg"
    install -Dm644 assets/veemeeter.service "$root/usr/lib/systemd/user/veemeeter.service"
    install -Dm644 README.md "$root/usr/share/doc/veemeeter/README.md"
    install -Dm644 LICENSE "$root/usr/share/licenses/veemeeter/LICENSE"
    version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
    tar -C "$root" --owner=0 --group=0 -czf "dist/veemeeter-$version-arch-x86_64.tar.gz" usr
    rm -rf "$root"
    ;;
esac
ls -la dist/

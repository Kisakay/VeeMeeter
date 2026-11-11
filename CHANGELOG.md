# Changelog

All notable changes to VeeMeeter. Versions follow [SemVer](https://semver.org/).

## [0.1.0] - 2026-11-11

The first release of VeeMeeter, a from-scratch Rust rewrite of PulseMeeter:
a Voicemeeter-style mixer and router that drives PipeWire natively. Say hi
to Mimi! (｡•̀ᴗ-)✧

### Highlights

- **Native PipeWire engine** — `veemeeter daemon` talks to libpipewire
  directly (no `pactl` / `pw-cli` subprocesses), owns every node and link it
  creates, and reconciles the graph on every change: unplug and replug a
  device and its routes come back by themselves. Stop the daemon and
  everything it created disappears.
- **Voicemeeter-style strips** — hardware inputs, virtual inputs, hardware
  outputs A1, A2… and virtual outputs (virtual mics) B1, B2…, with faders in
  dB (−∞ … +12), mute, VU meters, channel layouts (mono to 7.1) and one-click
  A/B routing.
- **Send levels** — right-click a route to give it its own level, e.g. music
  at −12 dB in your virtual mic only.
- **Effects per strip** — 5-band equalizer (live) and RNNoise noise
  suppression, through PipeWire's `filter-chain` inside the daemon.
- **Applications** — move any app's playback or recording to a strip; make a
  virtual strip the system default; desktop volume keys move its fader.
- **Native window** (egui/OpenGL, no browser) with Mimi, and a scriptable
  `veemeeter ctl` for keyboard shortcuts and status bars.

### Install

| Distribution | File | Command |
|---|---|---|
| Debian 13 | `veemeeter_0.1.0-1~debian13_amd64.deb` | `sudo apt install ./veemeeter_0.1.0-1~debian13_amd64.deb` |
| Ubuntu 24.04 LTS | `veemeeter_0.1.0-1~ubuntu24.04_amd64.deb` | `sudo apt install ./veemeeter_0.1.0-1~ubuntu24.04_amd64.deb` |
| Ubuntu (latest) | `veemeeter_0.1.0-1~ubuntu*_amd64.deb` | `sudo apt install ./veemeeter_0.1.0-1~ubuntu*_amd64.deb` |
| Fedora | `veemeeter-0.1.0-1.fedora*.x86_64.rpm` | `sudo dnf install ./veemeeter-0.1.0-1.fedora*.x86_64.rpm` |
| Arch Linux | `veemeeter-0.1.0-arch-x86_64.tar.gz` | `sudo tar -xzf veemeeter-0.1.0-arch-x86_64.tar.gz -C /` |
| NixOS / Nix | — | `nix run github:Kisakay/VeeMeeter/v0.1.0`, or the flake's NixOS / home-manager module |

Then open **VeeMeeter** from your menu (it starts its engine), or enable the
engine at login with `systemctl --user enable --now veemeeter`. Check your
download against `SHA256SUMS` (`sha256sum -c SHA256SUMS --ignore-missing`).

Noise suppression needs the RNNoise LADSPA plugin (`librnnoise_ladspa.so`,
from your distribution or
[noise-suppression-for-voice](https://github.com/werman/noise-suppression-for-voice/releases));
the Nix package already includes it.

### Known limits

- Needs a PipeWire session manager (WirePlumber), like any PipeWire desktop.
- A hardware output stays awake while something is routed to it.
- x86_64 packages only for now (the Nix flake also builds on aarch64).

[0.1.0]: https://github.com/Kisakay/VeeMeeter/releases/tag/v0.1.0

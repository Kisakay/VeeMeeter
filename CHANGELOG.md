# Changelog

All notable changes to VeeMeeter. Versions follow [SemVer](https://semver.org/).

## [0.2.0] - 2026-11-13

Mimi moves into your system tray, and VeeMeeter gets virtual cables.

### Added

- **System tray** — Mimi lives in the tray and dances to the music coming
  out of your outputs: she hops on the beats (higher when it's loud),
  equalizer bars pop up next to her, she cheers when it gets loud and naps
  after 30 seconds of silence. Menu: *Open Vee*, *Start on tray*, *Open
  GitHub*, *Report an issue*, *Close Vee*. New `veemeeter tray` command and
  `gui --no-tray` switch. Uses StatusNotifierItem (KDE, COSMIC, Xfce,
  waybar…; GNOME needs the AppIndicator extension).
- **Start on tray** — launch with only Mimi in the tray; set it from the tray
  menu or the new *Vee* menu in the window.
- **Virtual cables** (VB-Cable style) — `+ Add → Virtual cable` or
  `veemeeter ctl add c <name>`: one app plays into *<name>*, another records
  *<name> (output)*, with fader, mute, meter, effects and A/B routes.
- `veemeeter ctl device <id> <device|none>` picks a hardware strip's device.

### Changed

- Launching VeeMeeter while a window is open brings that window back
  instead of opening a second one.
- Changing a hardware strip's device gives it the device's channel layout
  (mono mic → mono strip).
- The strip, FX and send-level menus only close on a click outside them.

### Fixed

- First launch picks the real microphone, not the sound card's loopback
  monitor (seen on a Scarlett 2i2: the mic meter followed the desktop).

### Install

| Distribution | File | Command |
|---|---|---|
| Debian 13 | `veemeeter_0.2.0-1.debian13_amd64.deb` | `sudo apt install ./veemeeter_0.2.0-1.debian13_amd64.deb` |
| Ubuntu 24.04 LTS | `veemeeter_0.2.0-1.ubuntu24.04_amd64.deb` | `sudo apt install ./veemeeter_0.2.0-1.ubuntu24.04_amd64.deb` |
| Ubuntu (latest) | `veemeeter_0.2.0-1.ubuntu26.10_amd64.deb` | `sudo apt install ./veemeeter_0.2.0-1.ubuntu26.10_amd64.deb` |
| Fedora | `veemeeter-0.2.0-1.fedora45.x86_64.rpm` | `sudo dnf install ./veemeeter-0.2.0-1.fedora45.x86_64.rpm` |
| Arch Linux | `veemeeter-0.2.0-arch-x86_64.tar.gz` | `sudo tar -xzf veemeeter-0.2.0-arch-x86_64.tar.gz -C /` |
| NixOS / Nix | — | `nix run github:Kisakay/VeeMeeter/v0.2.0`, or the flake's NixOS / home-manager module |

Check your download with `sha256sum -c --ignore-missing SHA256SUMS`.

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
| Debian 13 | `veemeeter_0.1.0-1.debian13_amd64.deb` | `sudo apt install ./veemeeter_0.1.0-1.debian13_amd64.deb` |
| Ubuntu 24.04 LTS | `veemeeter_0.1.0-1.ubuntu24.04_amd64.deb` | `sudo apt install ./veemeeter_0.1.0-1.ubuntu24.04_amd64.deb` |
| Ubuntu (latest) | `veemeeter_0.1.0-1.ubuntu26.10_amd64.deb` | `sudo apt install ./veemeeter_0.1.0-1.ubuntu26.10_amd64.deb` |
| Fedora | `veemeeter-0.1.0-1.fedora45.x86_64.rpm` | `sudo dnf install ./veemeeter-0.1.0-1.fedora45.x86_64.rpm` |
| Arch Linux | `veemeeter-0.1.0-arch-x86_64.tar.gz` | `sudo tar -xzf veemeeter-0.1.0-arch-x86_64.tar.gz -C /` |
| NixOS / Nix | — | `nix run github:Kisakay/VeeMeeter/v0.1.0`, or the flake's NixOS / home-manager module |

Then open **VeeMeeter** from your menu (it starts its engine), or enable the
engine at login with `systemctl --user enable --now veemeeter`. Check your
download against `SHA256SUMS`. In 0.1.0 that file lists the `.deb` files
with a `~` that GitHub turned into a `.`, so check with
`sed 's/~/./' SHA256SUMS | sha256sum -c --ignore-missing`.

Noise suppression needs the RNNoise LADSPA plugin (`librnnoise_ladspa.so`,
from your distribution or
[noise-suppression-for-voice](https://github.com/werman/noise-suppression-for-voice/releases));
the Nix package already includes it.

### Known limits

- Needs a PipeWire session manager (WirePlumber), like any PipeWire desktop.
- A hardware output stays awake while something is routed to it.
- x86_64 packages only for now (the Nix flake also builds on aarch64).

[0.2.0]: https://github.com/Kisakay/VeeMeeter/releases/tag/v0.2.0
[0.1.0]: https://github.com/Kisakay/VeeMeeter/releases/tag/v0.1.0

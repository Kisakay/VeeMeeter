# Why VeeMeeter?

VeeMeeter is a Rust rewrite of [PulseMeeter](https://github.com/theRealCarneiro/pulsemeeter),
the Voicemeeter-like mixer for Linux. Same idea, same vocabulary (inputs,
outputs, A/B buses), different engine.

## At a glance

| | VeeMeeter |
|---|---|
| **Sound server access** | `libpipewire` directly, in-process. PulseMeeter drives the server through `pactl` / `pw-cli` subprocesses. |
| **Audio graph owner** | A small **daemon** (systemd user service). The window is only a remote control. |
| **Closing the window** | Routing keeps running. |
| **Stopping it** | Every VeeMeeter node and link vanishes cleanly. |
| **Hotplug** | Declarative reconcile: unplugged devices and their links come back on their own. |
| **UI** | Native egui window: no browser, no Electron. |
| **Scripting** | `veemeeter ctl` over a Unix socket (JSON lines). |
| **NixOS** | First-class flake: package, overlay, NixOS module, home-manager module. |
| **Language** | Rust, one binary. |

## Native PipeWire, no subprocesses

Spawning `pactl` for every fader move is slow and fragile: output parsing
breaks between versions, and state is polled instead of observed. VeeMeeter
links against `libpipewire`, creates its own nodes and links, and receives
graph events as they happen.

## A daemon that keeps routing alive

The engine is `veemeeter daemon`. The window and `ctl` are clients on
`$XDG_RUNTIME_DIR/veemeeter.sock`. Close the window, log out of your
compositor session, crash the GUI: your microphone still reaches Discord.
Because the daemon **owns** every object it creates, stopping it removes all
of them at once; nothing stays behind in `pw-dump`.

## Hotplug reconcile

VeeMeeter does not replay a list of commands. It keeps a description of what
*should* exist (strips, devices, routes) and continuously reconciles it with
what PipeWire actually has. Unplug the headset, plug it back: the missing
nodes and links are recreated, nothing to click.

## Light and native

One Rust binary and an immediate-mode egui window: no web runtime to
ship, start or keep in memory.

!!! mimi-think "Coming from PulseMeeter?"
    Recreate your strips with **+ Add** (or `veemeeter ctl add`), then toggle
    the same A/B routes. The concepts map one to one; see
    [Getting started](concepts.md).

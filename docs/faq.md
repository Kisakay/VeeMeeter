# Troubleshooting & FAQ

## Troubleshooting

??? failure "`veemeeter ctl`: is the daemon running?"
    `ctl` could not reach `$XDG_RUNTIME_DIR/veemeeter.sock`. Start the engine:

    ```sh
    systemctl --user start veemeeter   # with the service
    veemeeter daemon                   # or in a terminal
    ```

    Then check its logs: `journalctl --user -u veemeeter -e`.

??? failure "The window says *VeeMeeter is waking up…* forever"
    The window could not connect to (or start) the daemon. Run
    `veemeeter daemon` in a terminal to see the error. The most common cause
    is PipeWire itself not running: `systemctl --user status pipewire`.

??? failure "My virtual devices don't show up in other apps"
    - Check they exist: `veemeeter ctl status`.
    - PulseAudio apps (most of them) need `pipewire-pulse`:
      `systemctl --user status pipewire-pulse`.
    - Some apps only list devices at start-up; restart them.

??? failure "A hardware strip says *device offline* / *(unplugged)*"
    The device saved in `device = "…"` is not present. Plug it in (the strip
    reconnects by itself) or pick another one in the strip's device picker.

??? failure "Microphone is silent in Discord"
    1. In Discord, select **Virtual Mic** (your B1) as the input device.
    2. Make sure the **Microphone** strip has `B1` lit and is not muted.
    3. Watch the VU meter of `hi1` while speaking: no movement means the
       hardware strip uses the wrong device.

??? failure "I hear myself / echo"
    An input routed to the same output you listen on (for example the
    microphone sent to `A1`) is played back to you. Turn that A button off.

??? failure "Logs"
    The daemon logs to stderr (and the journal with the service). More
    detail: `RUST_LOG=debug veemeeter daemon`.

## FAQ

??? question "Does it work with PulseAudio or JACK?"
    It needs a **PipeWire** server. PulseAudio and JACK *clients* are fine
    through PipeWire's compatibility layers.

??? question "Does it change my system microphone volume?"
    No. Hardware inputs go through an internal hidden node, so the fader is
    VeeMeeter's own gain.

??? question "What happens if the daemon crashes?"
    Its nodes and links disappear with it, so apps fall back to your normal
    devices. The systemd user service can restart it; the routing comes back
    from `config.toml`.

??? question "Can I run it without systemd?"
    Yes: just run `veemeeter`, the window starts the daemon. Or start
    `veemeeter daemon` from your compositor's autostart.

??? question "Is there a Windows or macOS version?"
    No. VeeMeeter is Linux only, by design.

??? question "Who is the cute girl with headphones?"
    That's **Mimi**, our mascot. Meet her on the [branding page](branding.md).

!!! mimi-think "Still stuck?"
    Open an issue on [GitHub](https://github.com/Kisakay/VeeMeeter/issues)
    with the output of `veemeeter ctl status` and the daemon logs.

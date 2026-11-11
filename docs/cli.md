# CLI reference

```text
veemeeter                 open the mixer window (starts the daemon if needed)
veemeeter gui [--no-spawn]
veemeeter daemon [--config <path>]
veemeeter ctl [--json] <command> …
```

`veemeeter ctl` talks to the running daemon over a Unix socket at
`$XDG_RUNTIME_DIR/veemeeter.sock` (one JSON object per line). It is made for
keyboard shortcuts, status bars and scripts.

!!! note "Strip ids"
    Commands take strip **ids**, not names: `hi1`, `vi2`, `a1`, `b1`…
    `veemeeter ctl status` lists them.

## `veemeeter daemon`

Runs the audio engine in the foreground. Usually started by the systemd user
service. `--config <path>` overrides the default
`$XDG_CONFIG_HOME/veemeeter/config.toml`.

## `ctl` commands

### `status`

```sh
veemeeter ctl status
veemeeter ctl --json status   # machine-readable
```

Shows the PipeWire version, every strip (volume, routes, *muted* /
*default* / *device offline* flags), the available devices and the
applications currently playing or recording (with their numeric ids).

### `volume <id> <level|up|down> [step]`

```sh
veemeeter ctl volume vi1 0          # back to unity
veemeeter ctl volume vi1 -- -6      # −6 dB (put `--` before a negative number)
veemeeter ctl volume hi1 6dB        # a "dB" suffix is accepted
veemeeter ctl volume vi1 up         # 3 dB louder
veemeeter ctl volume vi1 down 1.5   # 1.5 dB quieter
veemeeter ctl volume b1 off         # fader to −∞
```

A number is always an **absolute** level, clamped to −60 (off) … +12 dB.
Use `up` / `down` (default step 3 dB) for relative changes, e.g. on
keyboard shortcuts.

### `mute <id> [on|off|toggle]`

```sh
veemeeter ctl mute hi1          # toggle (default)
veemeeter ctl mute hi1 on
```

`on`/`off` also accept `true`/`false`, `1`/`0`, `yes`/`no`.

### `route <from> <to> [on|off|toggle]`

```sh
veemeeter ctl route vi1 b1 on    # send Desktop into the virtual mic
veemeeter ctl route vi1 b1       # toggle
```

`<from>` is an input, `<to>` an output.

### `default <id>`

```sh
veemeeter ctl default vi2        # apps now play into "Voice chat"
```

Makes a virtual strip the system default output (virtual input) or default
microphone (virtual output).

### `add <kind> <name> [--device <node>] [--layout <layout>]`

```sh
veemeeter ctl add vi Music
veemeeter ctl add b Stream
veemeeter ctl add a Headset --device alsa_output.usb-…analog-stereo
veemeeter ctl add hi "Line in" --layout mono
```

| `kind` | Strip |
|---|---|
| `hi` | hardware input |
| `vi` | virtual input |
| `a` | hardware output (A bus) |
| `b` | virtual output (B bus) |

`--device` takes a PipeWire node name (see `ctl status`, *DEVICES*).
`--layout` is one of `mono`, `stereo`, `quad`, `5.1`, `7.1` (default stereo).

### `remove <id>` · `rename <id> <name>`

```sh
veemeeter ctl rename vi2 Discord
veemeeter ctl remove b2
```

### `move <app> <to>`

```sh
veemeeter ctl move 87 vi2        # move app stream #87 to "Voice chat"
```

`<app>` is the numeric id shown under *APPLICATIONS* in `ctl status`.

### `quit`

Stops the daemon; every VeeMeeter device disappears. With the systemd
service, prefer `systemctl --user stop veemeeter` (otherwise the service
manager may restart it, depending on its settings).

## Exit status

`0` on success. On failure `ctl` prints `veemeeter: <reason>` to stderr and
exits non-zero. If the daemon is not running you get a hint to start it with
`veemeeter daemon` or `systemctl --user start veemeeter`.

!!! mimi-think "Bind it to keys"
    Media keys are just commands. For example in Hyprland:
    `bind = , XF86AudioMicMute, exec, veemeeter ctl mute hi1`

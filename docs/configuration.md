# Configuration

VeeMeeter keeps its setup in **`~/.config/veemeeter/config.toml`**
(`$XDG_CONFIG_HOME/veemeeter/config.toml`). The daemon writes it every time
you change something in the window or with `ctl`, so most people never edit
it by hand.

!!! warning "Hand edits"
    The daemon reads the file at start-up. Edit it while the daemon is
    stopped (or restart it afterwards), otherwise your edit may be
    overwritten by the next change.

## Format

The file is a list of `[[strip]]` tables, in display order:

```toml title="~/.config/veemeeter/config.toml (first run)"
[[strip]]
id = "hi1"
kind = "hardware_in"
name = "Microphone"
channels = ["FL", "FR"]
volume_db = 0.0
mute = false
routes = ["b1"]

[[strip]]
id = "vi1"
kind = "virtual_in"
name = "Desktop"
channels = ["FL", "FR"]
volume_db = 0.0
mute = false
routes = ["a1"]
primary = true

[[strip]]
id = "vi2"
kind = "virtual_in"
name = "Voice chat"
channels = ["FL", "FR"]
volume_db = 0.0
mute = false
routes = ["a1"]

[[strip]]
id = "a1"
kind = "hardware_out"
name = "Speakers"
channels = ["FL", "FR"]
volume_db = 0.0
mute = false

[[strip]]
id = "b1"
kind = "virtual_out"
name = "Virtual Mic"
channels = ["FL", "FR"]
volume_db = 0.0
mute = false
primary = true
```

## Keys

| Key | Type | Applies to | Meaning |
|---|---|---|---|
| `id` | string | all | Unique id, lowercase letters and digits (≤ 16): `hi1`, `vi1`, `a1`, `b1`… Used by `ctl` and in `routes`. |
| `kind` | string | all | `hardware_in`, `virtual_in`, `hardware_out` or `virtual_out`. |
| `name` | string | all | Display name, also shown as the device name in other apps (≤ 64 characters). |
| `device` | string | hardware | PipeWire `node.name` of the real device. Missing = no device. |
| `channels` | list | all | Channel positions. Default `["FL", "FR"]`. |
| `volume_db` | float | all | Fader, −60 (off) … +12. Default `0.0`. |
| `mute` | bool | all | Default `false`. |
| `routes` | list | inputs | Ids of the outputs this input is sent to. |
| `primary` | bool | virtual | System default device. At most one virtual input and one virtual output. |

### Channel layouts

| Layout | `channels` |
|---|---|
| Mono | `["MONO"]` |
| Stereo | `["FL", "FR"]` |
| Quad | `["FL", "FR", "RL", "RR"]` |
| 5.1 | `["FL", "FR", "FC", "LFE", "RL", "RR"]` |
| 7.1 | `["FL", "FR", "FC", "LFE", "RL", "RR", "SL", "SR"]` |

## Validation

Invalid values are fixed when the file is loaded, never fatal:

- duplicate or invalid ids are dropped; at most **32** strips are kept;
- names are trimmed; an empty name becomes the id;
- `volume_db` is clamped to −60 … +12;
- `routes` pointing to unknown outputs are removed (and ignored on outputs);
- `device` is ignored on virtual strips; extra `primary = true` are cleared.

!!! tip "Finding device names"
    `veemeeter ctl status` prints a *DEVICES* section with each device's
    description and node name, ready to paste into `device = "…"`.

## Starting fresh

Stop the daemon, delete `config.toml`, start it again: the first-run setup
above is recreated.

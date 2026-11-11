//! `veemeeter ctl` : piloter le daemon depuis un terminal ou un script
//! (raccourcis clavier, barres d'état…).

use crate::ipc::Client;
use crate::model::{Command, Kind, Message, Snapshot, layout_name};
use anyhow::{Result, anyhow, bail};
use argh::FromArgs;
use std::time::Duration;

/// Control the running daemon.
#[derive(FromArgs)]
#[argh(subcommand, name = "ctl")]
pub struct CtlArgs {
    /// print the state as JSON instead of text
    #[argh(switch)]
    json: bool,
    #[argh(subcommand)]
    cmd: CtlCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum CtlCmd {
    Status(Status),
    Volume(Volume),
    Mute(Mute),
    Route(Route),
    Default(Default),
    Add(Add),
    Remove(Remove),
    Rename(Rename),
    Move(MoveApp),
    Eq(EqCmd),
    Denoise(DenoiseCmd),
    Quit(Quit),
}

/// Show strips, routes, devices and applications.
#[derive(FromArgs)]
#[argh(subcommand, name = "status")]
struct Status {}

/// Set a strip volume: an absolute level in dB (-60 = off … +12; write `-- -6`
/// for negative values), or `up` / `down` followed by an optional step (default 3 dB).
#[derive(FromArgs)]
#[argh(subcommand, name = "volume")]
struct Volume {
    #[argh(positional)]
    id: String,
    #[argh(positional)]
    level: String,
    #[argh(positional)]
    step: Option<f32>,
}

/// Mute a strip: on, off or toggle (default).
#[derive(FromArgs)]
#[argh(subcommand, name = "mute")]
struct Mute {
    #[argh(positional)]
    id: String,
    #[argh(positional, default = "String::from(\"toggle\")")]
    state: String,
}

/// Route an input to an output: on, off or toggle (default).
#[derive(FromArgs)]
#[argh(subcommand, name = "route")]
struct Route {
    #[argh(positional)]
    from: String,
    #[argh(positional)]
    to: String,
    #[argh(positional, default = "String::from(\"toggle\")")]
    state: String,
}

/// Make a virtual strip the system default device.
#[derive(FromArgs)]
#[argh(subcommand, name = "default")]
struct Default {
    #[argh(positional)]
    id: String,
}

/// Add a strip: kind is hi (hardware input), vi (virtual input), a (hardware output) or b (virtual output).
#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
struct Add {
    #[argh(positional)]
    kind: String,
    #[argh(positional)]
    name: String,
    /// hardware device node name (see `ctl status`)
    #[argh(option)]
    device: Option<String>,
    /// channel layout: mono, stereo, quad, 5.1, 7.1
    #[argh(option)]
    layout: Option<String>,
}

/// Remove a strip.
#[derive(FromArgs)]
#[argh(subcommand, name = "remove")]
struct Remove {
    #[argh(positional)]
    id: String,
}

/// Rename a strip.
#[derive(FromArgs)]
#[argh(subcommand, name = "rename")]
struct Rename {
    #[argh(positional)]
    id: String,
    #[argh(positional)]
    name: String,
}

/// Move an application stream (id from `ctl status`) to a strip.
#[derive(FromArgs)]
#[argh(subcommand, name = "move")]
struct MoveApp {
    #[argh(positional)]
    app: u32,
    #[argh(positional)]
    to: String,
}

/// Equalizer of a strip: on, off, toggle, or five gains in dB for
/// 80 Hz, 250 Hz, 1 kHz, 4 kHz and 10 kHz (e.g. `3,0,-2,1,0`; turns it on).
#[derive(FromArgs)]
#[argh(subcommand, name = "eq")]
struct EqCmd {
    #[argh(positional)]
    id: String,
    #[argh(positional, default = "String::from(\"toggle\")")]
    value: String,
}

/// Noise suppression (RNNoise) of a strip: on, off or toggle (default).
#[derive(FromArgs)]
#[argh(subcommand, name = "denoise")]
struct DenoiseCmd {
    #[argh(positional)]
    id: String,
    #[argh(positional, default = "String::from(\"toggle\")")]
    state: String,
}

/// Stop the daemon (all VeeMeeter devices disappear).
#[derive(FromArgs)]
#[argh(subcommand, name = "quit")]
struct Quit {}

pub fn run(args: CtlArgs) -> i32 {
    match exec(args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("veemeeter: {e:#}");
            1
        }
    }
}

fn toggle(s: &str) -> Result<Option<bool>> {
    Ok(match s {
        "on" | "true" | "1" | "yes" => Some(true),
        "off" | "false" | "0" | "no" => Some(false),
        "toggle" => None,
        _ => bail!("expected on, off or toggle"),
    })
}

fn exec(args: CtlArgs) -> Result<()> {
    let mut client = Client::connect().map_err(|e| {
        anyhow!("{e:#}\nis the daemon running? start it with `veemeeter daemon` or `systemctl --user start veemeeter`")
    })?;
    client.set_read_timeout(Some(Duration::from_secs(5)))?;
    client.send(&Command::Subscribe { meters: false })?;
    let state = loop {
        match client.recv()? {
            Message::State { state } => break state,
            Message::Error { text } => bail!(text),
            _ => {}
        }
    };
    let cmd = match args.cmd {
        CtlCmd::Status(_) => {
            if args.json {
                println!("{}", serde_json::to_string_pretty(&state)?);
            } else {
                print_status(&state);
            }
            return Ok(());
        }
        CtlCmd::Volume(v) => {
            let cur = state.strip(&v.id).ok_or_else(|| anyhow!("unknown strip {}", v.id))?.strip.volume_db;
            let db = volume_target(cur, &v.level, v.step)?;
            Command::Volume { id: v.id, db }
        }
        CtlCmd::Mute(m) => Command::Mute { id: m.id, mute: toggle(&m.state)? },
        CtlCmd::Route(r) => Command::Route { from: r.from, to: r.to, on: toggle(&r.state)? },
        CtlCmd::Default(d) => Command::Primary { id: d.id },
        CtlCmd::Add(a) => {
            let kind = Kind::parse(&a.kind).ok_or_else(|| anyhow!("kind must be hi, vi, a or b"))?;
            let channels = match a.layout.as_deref() {
                None => None,
                Some(l) => Some(
                    crate::model::LAYOUTS
                        .iter()
                        .find(|(n, _)| n.eq_ignore_ascii_case(l))
                        .ok_or_else(|| anyhow!("unknown layout {l}"))?
                        .1
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                ),
            };
            Command::Add { kind, name: a.name, device: a.device, channels }
        }
        CtlCmd::Remove(r) => Command::Remove { id: r.id },
        CtlCmd::Rename(r) => Command::Rename { id: r.id, name: r.name },
        CtlCmd::Move(m) => Command::MoveApp { app: m.app, to: m.to },
        CtlCmd::Eq(e) => {
            let fx = &state.strip(&e.id).ok_or_else(|| anyhow!("unknown strip {}", e.id))?.strip.fx;
            match e.value.as_str() {
                "on" | "off" | "toggle" | "true" | "false" | "1" | "0" | "yes" | "no" => {
                    Command::Eq { id: e.id, enabled: Some(toggle(&e.value)?.unwrap_or(!fx.eq)), gains: None }
                }
                list => Command::Eq { id: e.id, enabled: Some(true), gains: Some(parse_gains(list)?) },
            }
        }
        CtlCmd::Denoise(d) => {
            if !state.denoise_available {
                bail!("noise suppression needs the RNNoise LADSPA plugin (librnnoise_ladspa.so), see the docs");
            }
            Command::Denoise { id: d.id, on: toggle(&d.state)? }
        }
        CtlCmd::Quit(_) => {
            client.send(&Command::Quit)?;
            return Ok(());
        }
    };
    client.send(&cmd)?;
    client.send(&Command::Ping { tag: 1 })?;
    loop {
        match client.recv()? {
            Message::Error { text } => bail!(text),
            Message::Pong { .. } => return Ok(()),
            _ => {}
        }
    }
}

/// `3,0,-2,1,0` → cinq gains en dB.
fn parse_gains(list: &str) -> Result<[f32; 5]> {
    let v: Vec<f32> = list
        .split(',')
        .map(|g| g.trim().trim_end_matches("dB").parse::<f32>())
        .collect::<Result<_, _>>()
        .map_err(|_| anyhow!("expected on, off, toggle or five gains like 3,0,-2,1,0"))?;
    v.try_into().map_err(|_| anyhow!("the equalizer has exactly five bands"))
}

/// Niveau voulu : `up`/`down [pas]` relatifs, sinon un niveau absolu en dB
/// (`-6`, `+3`, `0dB`, `-inf`).
fn volume_target(current: f32, level: &str, step: Option<f32>) -> Result<f32> {
    let step = step.unwrap_or(3.0).abs();
    Ok(match level {
        "up" => current + step,
        "down" => current - step,
        "-inf" | "off" => crate::model::MIN_DB,
        _ => {
            if step != 3.0 {
                bail!("a step is only valid with up or down");
            }
            level
                .trim_end_matches("dB")
                .trim_end_matches("db")
                .parse::<f32>()
                .map_err(|_| anyhow!("expected a level in dB, up or down"))?
        }
    })
}

fn print_status(s: &Snapshot) {
    println!("PipeWire {}", s.pipewire_version);
    for (title, input) in [("INPUTS", true), ("OUTPUTS", false)] {
        println!("\n{title}");
        for st in s.strips.iter().filter(|st| st.strip.kind.is_input() == input) {
            let x = &st.strip;
            let label = if input { x.id.clone() } else { format!("{} ({})", x.id, s.bus_label(&x.id)) };
            let vol = if x.volume_db <= crate::model::MIN_DB {
                "  -inf".to_string()
            } else {
                format!("{:+6.1}", x.volume_db)
            };
            let mut flags = vec![];
            if x.fx.eq {
                flags.push("eq");
            }
            if x.fx.denoise {
                flags.push("denoise");
            }
            if x.mute {
                flags.push("muted");
            }
            if st.is_default {
                flags.push("default");
            }
            if !st.ready {
                flags.push("not ready");
            }
            if x.kind.is_hardware() && !st.online {
                flags.push("device offline");
            }
            println!(
                "  {label:<10} {:<20} {vol} dB  {:<7} {}{}",
                x.name,
                layout_name(&x.channels),
                if input && !x.routes.is_empty() {
                    format!("→ {} ", x.routes.iter().map(|r| s.bus_label(r)).collect::<Vec<_>>().join(" "))
                } else {
                    String::new()
                },
                if flags.is_empty() { String::new() } else { format!("[{}]", flags.join(", ")) }
            );
            if let Some(d) = &x.device {
                let desc = s
                    .devices
                    .iter()
                    .find(|dv| &dv.name == d)
                    .map(|dv| dv.description.as_str())
                    .unwrap_or("(unplugged)");
                println!("  {:<10} └ {desc}", "");
            }
        }
    }
    println!("\nDEVICES");
    for d in &s.devices {
        println!("  {} {:<40} {}", if d.capture { "in " } else { "out" }, d.description, d.name);
    }
    if !s.apps.is_empty() {
        println!("\nAPPLICATIONS");
        for a in &s.apps {
            println!(
                "  {:>5} {} {:<24} {:<30} → {}",
                a.id,
                if a.playback { "▶" } else { "●" },
                a.name,
                a.media.chars().take(30).collect::<String>(),
                a.target_strip.clone().or(a.target.clone()).unwrap_or_else(|| "-".into())
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::volume_target;

    #[test]
    fn absolute_and_relative_levels() {
        assert_eq!(volume_target(-10.0, "-6", None).unwrap(), -6.0);
        assert_eq!(volume_target(-10.0, "+3", None).unwrap(), 3.0);
        assert_eq!(volume_target(-10.0, "0dB", None).unwrap(), 0.0);
        assert_eq!(volume_target(-10.0, "up", None).unwrap(), -7.0);
        assert_eq!(volume_target(-10.0, "down", Some(5.0)).unwrap(), -15.0);
        assert_eq!(volume_target(-10.0, "off", None).unwrap(), crate::model::MIN_DB);
        assert!(volume_target(0.0, "-6", Some(2.0)).is_err());
        assert!(volume_target(0.0, "loud", None).is_err());
    }

    #[test]
    fn eq_gain_lists() {
        assert_eq!(super::parse_gains("3,0,-2.5,1dB,0").unwrap(), [3.0, 0.0, -2.5, 1.0, 0.0]);
        assert!(super::parse_gains("1,2,3").is_err());
        assert!(super::parse_gains("a,b,c,d,e").is_err());
    }
}

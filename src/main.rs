//! VeeMeeter — table de mixage et routeur audio PipeWire natif pour Linux.

mod config;
mod ctl;
mod engine;
mod gui;
mod ipc;
mod model;

use argh::FromArgs;
use std::path::PathBuf;

/// VeeMeeter: native PipeWire audio mixer and router.
#[derive(FromArgs)]
struct Args {
    #[argh(subcommand)]
    cmd: Option<Sub>,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Sub {
    Daemon(DaemonArgs),
    Gui(GuiArgs),
    Tray(TrayArgs),
    Ctl(ctl::CtlArgs),
}

/// Run the audio engine (keeps the routing alive; usually a systemd user service).
#[derive(FromArgs)]
#[argh(subcommand, name = "daemon")]
struct DaemonArgs {
    /// config file (default: $XDG_CONFIG_HOME/veemeeter/config.toml)
    #[argh(option)]
    config: Option<PathBuf>,
}

/// Open the mixer window (default; starts the daemon if needed).
#[derive(FromArgs)]
#[argh(subcommand, name = "gui")]
struct GuiArgs {
    /// do not start the daemon automatically
    #[argh(switch)]
    no_spawn: bool,
    /// do not put Mimi in the system tray
    #[argh(switch)]
    no_tray: bool,
}

/// Show Mimi in the system tray only (no window); she dances to the music.
#[derive(FromArgs)]
#[argh(subcommand, name = "tray")]
struct TrayArgs {
    /// do not start the daemon automatically
    #[argh(switch)]
    no_spawn: bool,
    /// wait for a system tray to appear instead of exiting (login autostart)
    #[argh(switch)]
    wait: bool,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args: Args = argh::from_env();
    let code = match args.cmd {
        Some(Sub::Daemon(a)) => match engine::run(a.config.unwrap_or_else(config::default_path)) {
            Ok(code) => code,
            Err(e) => {
                log::error!("{e:#}");
                1
            }
        },
        Some(Sub::Ctl(a)) => ctl::run(a),
        Some(Sub::Gui(a)) => gui::launch(!a.no_spawn, !a.no_tray),
        Some(Sub::Tray(a)) => gui::tray::run(!a.no_spawn, a.wait).unwrap_or_else(|e| {
            log::error!("{e}");
            1
        }),
        None => gui::launch(true, true),
    };
    std::process::exit(code);
}

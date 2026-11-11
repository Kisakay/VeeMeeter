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
        Some(Sub::Gui(a)) => gui::run(!a.no_spawn),
        None => gui::run(true),
    };
    std::process::exit(code);
}

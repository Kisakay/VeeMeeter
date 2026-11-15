//! Interface : la fenêtre de mixage (libcosmic, `window`) et Mimi dans la
//! zone de notification (`tray`), deux clients du daemon.

mod instance;
mod link;
mod mascot;
mod settings;
mod theme;
pub mod tray;
mod window;

/// Point d'entrée de `veemeeter` / `veemeeter gui` : ramène la fenêtre si
/// elle est déjà ouverte, sinon démarre dans le tray (option « Start on
/// tray ») ou ouvre la fenêtre, avec le tray à côté.
pub fn launch(spawn: bool, with_tray: bool) -> i32 {
    if instance::tell_gui(instance::Order::Show) {
        return 0;
    }
    settings::Settings::load().sync_autostart();
    if with_tray && settings::Settings::load().start_on_tray && !instance::running("tray") {
        match tray::run(spawn, false) {
            Ok(code) => return code,
            Err(e) => log::warn!("{e}; opening the window instead"),
        }
    }
    if with_tray && !instance::running("tray") {
        spawn_tray(spawn);
    }
    run(spawn)
}

/// Le tray vit dans son propre processus : il survit à la fenêtre.
fn spawn_tray(spawn: bool) {
    use std::os::unix::process::CommandExt;
    use std::process::{Command as Proc, Stdio};
    let Ok(exe) = std::env::current_exe() else { return };
    let mut cmd = Proc::new(exe);
    cmd.arg("tray");
    if !spawn {
        cmd.arg("--no-spawn");
    }
    let r = cmd.stdin(Stdio::null()).stdout(Stdio::null()).process_group(0).spawn();
    match r {
        // Récolté en arrière-plan pour ne pas laisser de zombie si la fenêtre dure.
        Ok(mut c) => drop(std::thread::spawn(move || c.wait())),
        Err(e) => log::warn!("cannot start the tray: {e}"),
    }
}

fn run(spawn: bool) -> i32 {
    let Some(listener) = instance::claim("gui") else {
        // Une autre fenêtre vient de s'ouvrir entre-temps.
        instance::tell_gui(instance::Order::Show);
        return 0;
    };
    window::listen_orders(listener);
    window::run(spawn)
}

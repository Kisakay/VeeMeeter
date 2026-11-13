//! Préférences de l'interface (`$XDG_CONFIG_HOME/veemeeter/gui.toml`), à part
//! de la config du daemon : elles ne regardent que la fenêtre et le tray.
//! « Start on tray » pose aussi une entrée de démarrage de session
//! (`$XDG_CONFIG_HOME/autostart/veemeeter.desktop`) : Mimi arrive dans le
//! tray à l'ouverture de session.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// Au lancement, ne montrer que l'icône du tray (pas de fenêtre).
    pub start_on_tray: bool,
}

fn path() -> PathBuf {
    crate::config::default_path().with_file_name("gui.toml")
}

/// `$XDG_CONFIG_HOME/autostart/veemeeter.desktop`.
fn autostart_path() -> PathBuf {
    let config = crate::config::default_path();
    let base = config.parent().and_then(|d| d.parent()).map(PathBuf::from).unwrap_or_default();
    base.join("autostart").join("veemeeter.desktop")
}

/// Marque nos entrées : on ne retire jamais un fichier écrit par quelqu'un d'autre.
const AUTOSTART_MARK: &str = "X-VeeMeeter-StartOnTray=true";

/// `veemeeter` s'il est dans le PATH (survit aux mises à jour, chemins Nix
/// compris), sinon le chemin de ce binaire.
fn exec_command() -> String {
    let in_path =
        std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("veemeeter").is_file()));
    let exe = if in_path {
        "veemeeter".to_string()
    } else {
        std::env::current_exe().map(|p| format!("\"{}\"", p.display())).unwrap_or_else(|_| "veemeeter".into())
    };
    format!("{exe} tray --wait")
}

fn autostart_entry() -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=VeeMeeter (tray)\n\
         Comment=Mimi in the system tray; click her to open the mixer\n\
         Exec={}\nIcon=veemeeter\nTerminal=false\nNoDisplay=true\n\
         X-GNOME-Autostart-enabled=true\n{AUTOSTART_MARK}\n",
        exec_command()
    )
}

impl Settings {
    pub fn load() -> Settings {
        std::fs::read_to_string(path()).ok().and_then(|t| toml::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let p = path();
        let res = p
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&p, toml::to_string_pretty(self).unwrap_or_default()));
        if let Err(e) = res {
            log::warn!("cannot save {}: {e}", p.display());
        }
    }

    pub fn set_start_on_tray(on: bool) {
        let mut s = Settings::load();
        if s.start_on_tray != on {
            s.start_on_tray = on;
            s.save();
        }
        s.sync_autostart();
    }

    /// Met l'entrée de démarrage de session en accord avec l'option.
    pub fn sync_autostart(&self) {
        let p = autostart_path();
        let current = std::fs::read_to_string(&p).ok();
        let res = if self.start_on_tray {
            let want = autostart_entry();
            // Une entrée écrite à la main (sans notre marque) est laissée telle quelle.
            if current.as_deref().is_some_and(|c| c == want || !c.contains(AUTOSTART_MARK)) {
                return;
            }
            p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&p, want))
        } else if current.is_some_and(|c| c.contains(AUTOSTART_MARK)) {
            std::fs::remove_file(&p)
        } else {
            return;
        };
        if let Err(e) = res {
            log::warn!("cannot update {}: {e}", p.display());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn autostart_entry_starts_the_tray() {
        let e = super::autostart_entry();
        assert!(e.starts_with("[Desktop Entry]\n"));
        assert!(e.lines().any(|l| l.starts_with("Exec=") && l.ends_with(" tray --wait")));
        assert!(e.contains(super::AUTOSTART_MARK));
    }
}

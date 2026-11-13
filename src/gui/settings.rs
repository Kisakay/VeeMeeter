//! Préférences de l'interface (`$XDG_CONFIG_HOME/veemeeter/gui.toml`), à part
//! de la config du daemon : elles ne regardent que la fenêtre et le tray.

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
    }
}

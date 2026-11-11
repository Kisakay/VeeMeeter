//! Configuration persistée (`$XDG_CONFIG_HOME/veemeeter/config.toml`) et
//! application des commandes qui la modifient (logique pure, sans PipeWire).

use crate::model::{Command, EQ_RANGE, Kind, MAX_DB, MIN_DB, Strip, stereo};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Nombre maximal de strips (garde-fou : chaque strip = un nœud PipeWire).
pub const MAX_STRIPS: usize = 32;
const MAX_NAME: usize = 64;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Config {
    #[serde(default, rename = "strip")]
    pub strips: Vec<Strip>,
}

pub fn default_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("veemeeter").join("config.toml")
}

impl Config {
    /// Configuration de premier lancement : un micro, deux entrées virtuelles
    /// (bureau + voix), une sortie physique, un micro virtuel.
    pub fn first_run() -> Config {
        let mk = |id: &str, kind, name: &str| Strip {
            id: id.into(),
            kind,
            name: name.into(),
            device: None,
            channels: stereo(),
            volume_db: 0.0,
            mute: false,
            routes: vec![],
            primary: false,
            fx: Default::default(),
        };
        let mut hi = mk("hi1", Kind::HardwareIn, "Microphone");
        hi.routes = vec!["b1".into()];
        let mut desktop = mk("vi1", Kind::VirtualIn, "Desktop");
        desktop.routes = vec!["a1".into()];
        desktop.primary = true;
        let mut voice = mk("vi2", Kind::VirtualIn, "Voice chat");
        voice.routes = vec!["a1".into()];
        let a1 = mk("a1", Kind::HardwareOut, "Speakers");
        let mut b1 = mk("b1", Kind::VirtualOut, "Virtual Mic");
        b1.primary = true;
        Config { strips: vec![hi, desktop, voice, a1, b1] }
    }

    pub fn load(path: &Path) -> Result<Option<Config>> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let mut cfg: Config =
                    toml::from_str(&text).with_context(|| format!("invalid config {}", path.display()))?;
                cfg.sanitize();
                Ok(Some(cfg))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    /// Écriture atomique (fichier temporaire + rename).
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = format!(
            "# VeeMeeter configuration — written by the daemon, edits are picked up on restart.\n\n{}",
            toml::to_string_pretty(self)?
        );
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Répare une config éditée à la main : ids uniques, routes valides,
    /// bornes, un seul `primary` par type.
    pub fn sanitize(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.strips.retain(|s| valid_id(&s.id) && seen.insert(s.id.clone()));
        self.strips.truncate(MAX_STRIPS);
        let outputs: Vec<String> = self.strips.iter().filter(|s| !s.kind.is_input()).map(|s| s.id.clone()).collect();
        let mut primary_seen = [false; 2];
        for s in &mut self.strips {
            s.name = clean_name(&s.name).unwrap_or_else(|| s.id.clone());
            s.volume_db = clamp_db(s.volume_db);
            s.fx.gains = s.fx.gains.map(clamp_eq);
            if s.channels.is_empty() || s.channels.len() > 32 {
                s.channels = stereo();
            }
            if !s.kind.is_hardware() {
                s.device = None;
            }
            if s.kind.is_input() {
                s.routes.retain(|r| outputs.contains(r));
                s.routes.dedup();
            } else {
                s.routes.clear();
            }
            let slot = match s.kind {
                Kind::VirtualIn => Some(0),
                Kind::VirtualOut => Some(1),
                _ => None,
            };
            match slot {
                Some(i) if s.primary && !primary_seen[i] => primary_seen[i] = true,
                _ => s.primary = false,
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<&Strip> {
        self.strips.iter().find(|s| s.id == id)
    }

    fn get_mut(&mut self, id: &str) -> Result<&mut Strip, String> {
        self.strips.iter_mut().find(|s| s.id == id).ok_or_else(|| format!("unknown strip {id}"))
    }

    fn next_id(&self, kind: Kind) -> String {
        (1..).map(|n| format!("{}{n}", kind.prefix())).find(|id| self.get(id).is_none()).expect("infinite range")
    }

    /// Applique une commande de configuration. `Ok(true)` = la config a changé.
    /// Les commandes qui ne touchent pas la config renvoient `Ok(false)`.
    pub fn apply(&mut self, cmd: &Command) -> Result<bool, String> {
        match cmd {
            Command::Add { kind, name, device, channels } => {
                if self.strips.len() >= MAX_STRIPS {
                    return Err(format!("at most {MAX_STRIPS} strips"));
                }
                let name = clean_name(name).ok_or("invalid name")?;
                let channels = match channels {
                    Some(c) => check_channels(c)?,
                    None => stereo(),
                };
                let id = self.next_id(*kind);
                // Insère après le dernier strip du même type pour garder les groupes.
                let pos = self.strips.iter().rposition(|s| s.kind == *kind).map(|p| p + 1).unwrap_or(
                    // Entrées avant sorties, sinon en fin de liste.
                    if kind.is_input() {
                        self.strips.iter().position(|s| !s.kind.is_input()).unwrap_or(self.strips.len())
                    } else {
                        self.strips.len()
                    },
                );
                self.strips.insert(
                    pos,
                    Strip {
                        id,
                        kind: *kind,
                        name,
                        device: if kind.is_hardware() { device.clone() } else { None },
                        channels,
                        volume_db: 0.0,
                        mute: false,
                        routes: vec![],
                        primary: false,
                        fx: Default::default(),
                    },
                );
                Ok(true)
            }
            Command::Remove { id } => {
                let before = self.strips.len();
                self.strips.retain(|s| &s.id != id);
                if self.strips.len() == before {
                    return Err(format!("unknown strip {id}"));
                }
                for s in &mut self.strips {
                    s.routes.retain(|r| r != id);
                }
                Ok(true)
            }
            Command::Rename { id, name } => {
                let name = clean_name(name).ok_or("invalid name")?;
                let s = self.get_mut(id)?;
                Ok(std::mem::replace(&mut s.name, name.clone()) != name)
            }
            Command::SetDevice { id, device } => {
                let s = self.get_mut(id)?;
                if !s.kind.is_hardware() {
                    return Err("only hardware strips have a device".into());
                }
                Ok(std::mem::replace(&mut s.device, device.clone()) != *device)
            }
            Command::SetChannels { id, channels } => {
                let channels = check_channels(channels)?;
                let s = self.get_mut(id)?;
                Ok(std::mem::replace(&mut s.channels, channels.clone()) != channels)
            }
            Command::Volume { id, db } => {
                if !db.is_finite() {
                    return Err("invalid volume".into());
                }
                let s = self.get_mut(id)?;
                let db = clamp_db(*db);
                Ok(std::mem::replace(&mut s.volume_db, db) != db)
            }
            Command::Mute { id, mute } => {
                let s = self.get_mut(id)?;
                let m = mute.unwrap_or(!s.mute);
                Ok(std::mem::replace(&mut s.mute, m) != m)
            }
            Command::Route { from, to, on } => {
                match self.get(to) {
                    Some(t) if !t.kind.is_input() => {}
                    Some(_) => return Err(format!("{to} is not an output")),
                    None => return Err(format!("unknown strip {to}")),
                }
                let s = self.get_mut(from)?;
                if !s.kind.is_input() {
                    return Err(format!("{from} is not an input"));
                }
                let has = s.routes.contains(to);
                let want = on.unwrap_or(!has);
                if want == has {
                    return Ok(false);
                }
                if want {
                    s.routes.push(to.clone());
                } else {
                    s.routes.retain(|r| r != to);
                }
                Ok(true)
            }
            Command::Primary { id } => {
                let kind = self.get(id).ok_or_else(|| format!("unknown strip {id}"))?.kind;
                if kind.is_hardware() {
                    return Err("only virtual strips can be the system default".into());
                }
                let mut changed = false;
                for s in self.strips.iter_mut().filter(|s| s.kind == kind) {
                    let p = s.id == *id;
                    changed |= s.primary != p;
                    s.primary = p;
                }
                Ok(changed)
            }
            Command::Reorder { id, delta } => {
                let i = self.strips.iter().position(|s| &s.id == id).ok_or_else(|| format!("unknown strip {id}"))?;
                let kind = self.strips[i].kind;
                // Ne permute qu'avec un voisin du même groupe (entrées / sorties).
                let group: Vec<usize> =
                    (0..self.strips.len()).filter(|&j| self.strips[j].kind.is_input() == kind.is_input()).collect();
                let gi = group.iter().position(|&j| j == i).expect("in group") as i64;
                let target = (gi + *delta as i64).clamp(0, group.len() as i64 - 1) as usize;
                if group[target] == i {
                    return Ok(false);
                }
                let s = self.strips.remove(i);
                self.strips.insert(group[target], s);
                Ok(true)
            }
            Command::Eq { id, enabled, gains } => {
                let s = self.get_mut(id)?;
                let before = s.fx.clone();
                if let Some(e) = enabled {
                    s.fx.eq = *e;
                }
                if let Some(g) = gains {
                    if g.iter().any(|v| !v.is_finite()) {
                        return Err("invalid gain".into());
                    }
                    s.fx.gains = g.map(clamp_eq);
                }
                Ok(s.fx != before)
            }
            Command::Denoise { id, on } => {
                let s = self.get_mut(id)?;
                let want = on.unwrap_or(!s.fx.denoise);
                Ok(std::mem::replace(&mut s.fx.denoise, want) != want)
            }
            Command::Subscribe { .. } | Command::MoveApp { .. } | Command::Ping { .. } | Command::Quit => Ok(false),
        }
    }
}

/// Gain d'égaliseur borné, arrondi au dixième de dB.
fn clamp_eq(g: f32) -> f32 {
    if g.is_finite() { (g * 10.0).round().clamp(-EQ_RANGE * 10.0, EQ_RANGE * 10.0) / 10.0 } else { 0.0 }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 16 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

fn clamp_db(db: f32) -> f32 {
    if db.is_finite() { db.clamp(MIN_DB, MAX_DB) } else { 0.0 }
}

pub fn clean_name(name: &str) -> Option<String> {
    let n: String = name.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string();
    if n.is_empty() || n.chars().count() > MAX_NAME { None } else { Some(n) }
}

fn check_channels(c: &[String]) -> Result<Vec<String>, String> {
    if c.is_empty() || c.len() > 32 {
        return Err("1 to 32 channels".into());
    }
    let mut out = Vec::new();
    for ch in c {
        let ch = ch.trim().to_ascii_uppercase();
        if ch.is_empty() || ch.len() > 8 || !ch.bytes().all(|b| b.is_ascii_alphanumeric()) || out.contains(&ch) {
            return Err(format!("invalid channel {ch:?}"));
        }
        out.push(ch);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_is_sane_and_roundtrips() {
        let mut c = Config::first_run();
        let before = c.clone();
        c.sanitize();
        assert_eq!(c, before);
        let text = toml::to_string_pretty(&c).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn add_keeps_groups_and_ids() {
        let mut c = Config::first_run();
        c.apply(&Command::Add { kind: Kind::HardwareOut, name: "Headset".into(), device: None, channels: None })
            .unwrap();
        c.apply(&Command::Add { kind: Kind::VirtualIn, name: "Music".into(), device: None, channels: None }).unwrap();
        let ids: Vec<_> = c.strips.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["hi1", "vi1", "vi2", "vi3", "a1", "a2", "b1"]);
    }

    #[test]
    fn remove_drops_routes() {
        let mut c = Config::first_run();
        assert!(c.apply(&Command::Remove { id: "a1".into() }).unwrap());
        assert!(c.strips.iter().all(|s| !s.routes.contains(&"a1".to_string())));
    }

    #[test]
    fn route_rules() {
        let mut c = Config::first_run();
        assert!(c.apply(&Command::Route { from: "a1".into(), to: "b1".into(), on: Some(true) }).is_err());
        assert!(c.apply(&Command::Route { from: "vi1".into(), to: "vi2".into(), on: Some(true) }).is_err());
        assert!(c.apply(&Command::Route { from: "vi1".into(), to: "b1".into(), on: None }).unwrap());
        assert!(!c.apply(&Command::Route { from: "vi1".into(), to: "b1".into(), on: Some(true) }).unwrap());
        assert!(c.apply(&Command::Route { from: "vi1".into(), to: "b1".into(), on: None }).unwrap());
        assert_eq!(c.get("vi1").unwrap().routes, ["a1"]);
    }

    #[test]
    fn primary_is_exclusive_per_kind() {
        let mut c = Config::first_run();
        c.apply(&Command::Primary { id: "vi2".into() }).unwrap();
        assert!(!c.get("vi1").unwrap().primary && c.get("vi2").unwrap().primary);
        assert!(c.get("b1").unwrap().primary);
        assert!(c.apply(&Command::Primary { id: "hi1".into() }).is_err());
    }

    #[test]
    fn reorder_stays_in_group() {
        let mut c = Config::first_run();
        // Déjà dernière entrée : ne passe pas dans le groupe des sorties.
        assert!(!c.apply(&Command::Reorder { id: "vi2".into(), delta: 10 }).unwrap());
        assert!(c.apply(&Command::Reorder { id: "vi2".into(), delta: -2 }).unwrap());
        let ids: Vec<_> = c.strips.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["vi2", "hi1", "vi1", "a1", "b1"]);
    }

    #[test]
    fn sanitize_fixes_hand_edits() {
        let mut c: Config = toml::from_str(
            r#"
            [[strip]]
            id = "vi1"
            kind = "virtual_in"
            name = "  A  "
            volume_db = 99.0
            routes = ["zz", "a1"]
            primary = true
            [[strip]]
            id = "vi1"
            kind = "virtual_in"
            name = "dup"
            [[strip]]
            id = "a1"
            kind = "hardware_out"
            name = "Out"
            primary = true
            "#,
        )
        .unwrap();
        c.sanitize();
        assert_eq!(c.strips.len(), 2);
        assert_eq!(c.strips[0].name, "A");
        assert_eq!(c.strips[0].volume_db, MAX_DB);
        assert_eq!(c.strips[0].routes, ["a1"]);
        assert!(!c.strips[1].primary);
    }

    #[test]
    fn eq_and_denoise() {
        let mut c = Config::first_run();
        let gains = Some([3.0, 0.0, -99.0, 0.0, 1.25]);
        assert!(c.apply(&Command::Eq { id: "hi1".into(), enabled: Some(true), gains }).unwrap());
        let fx = &c.get("hi1").unwrap().fx;
        assert!(fx.eq);
        assert_eq!(fx.gains, [3.0, 0.0, -EQ_RANGE, 0.0, 1.3]);
        assert!(!c.apply(&Command::Eq { id: "hi1".into(), enabled: Some(true), gains: None }).unwrap());
        assert!(c.apply(&Command::Denoise { id: "hi1".into(), on: None }).unwrap());
        assert!(c.get("hi1").unwrap().fx.denoise);
        // Les effets par défaut ne sont pas écrits dans le fichier.
        assert!(!toml::to_string(c.get("vi1").unwrap()).unwrap().contains("fx"));
    }

    #[test]
    fn channels_are_validated() {
        let mut c = Config::first_run();
        assert!(c.apply(&Command::SetChannels { id: "vi1".into(), channels: vec!["fl".into(), "fl".into()] }).is_err());
        assert!(c.apply(&Command::SetChannels { id: "vi1".into(), channels: vec!["mono".into()] }).unwrap());
        assert_eq!(c.get("vi1").unwrap().channels, ["MONO"]);
    }
}

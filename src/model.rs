//! Types partagés entre le daemon, la GUI et `veemeeter ctl`.
//!
//! Vocabulaire (celui de Voicemeeter) :
//! - **entrées** (strips) : `HardwareIn` (micro, carte son…) et `VirtualIn`
//!   (périphérique de sortie virtuel dans lequel les applications jouent) ;
//! - **sorties** (bus) : `HardwareOut` (A1, A2… : enceintes, casque) et
//!   `VirtualOut` (B1, B2… : micro virtuel que les applications enregistrent).
//!
//! Une entrée est routée vers zéro, une ou plusieurs sorties.

use serde::{Deserialize, Serialize};

/// Gain minimal du fader : en dessous, le canal est coupé (−∞).
pub const MIN_DB: f32 = -60.0;
/// Gain maximal du fader.
pub const MAX_DB: f32 = 12.0;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    HardwareIn,
    VirtualIn,
    HardwareOut,
    VirtualOut,
}

impl Kind {
    pub fn is_input(self) -> bool {
        matches!(self, Kind::HardwareIn | Kind::VirtualIn)
    }

    pub fn is_hardware(self) -> bool {
        matches!(self, Kind::HardwareIn | Kind::HardwareOut)
    }

    /// Préfixe des identifiants (`hi1`, `vi2`, `a1`, `b1`).
    pub fn prefix(self) -> &'static str {
        match self {
            Kind::HardwareIn => "hi",
            Kind::VirtualIn => "vi",
            Kind::HardwareOut => "a",
            Kind::VirtualOut => "b",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::HardwareIn => "Hardware input",
            Kind::VirtualIn => "Virtual input",
            Kind::HardwareOut => "Hardware output",
            Kind::VirtualOut => "Virtual output",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "hi" | "hardware-in" | "hardware_in" | "mic" => Kind::HardwareIn,
            "vi" | "virtual-in" | "virtual_in" => Kind::VirtualIn,
            "a" | "hardware-out" | "hardware_out" => Kind::HardwareOut,
            "b" | "virtual-out" | "virtual_out" => Kind::VirtualOut,
            _ => return None,
        })
    }
}

/// Une entrée ou une sortie de la table de mixage, telle que persistée.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Strip {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    /// Périphérique physique (`node.name` PipeWire) — strips hardware seulement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// Positions des canaux (`FL`, `FR`, `MONO`…).
    #[serde(default = "stereo")]
    pub channels: Vec<String>,
    #[serde(default)]
    pub volume_db: f32,
    #[serde(default)]
    pub mute: bool,
    /// Sorties alimentées par cette entrée (ids), entrées seulement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<String>,
    /// Périphérique par défaut du système (strips virtuels seulement).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub primary: bool,
    /// Effets du strip : égaliseur et suppression de bruit.
    #[serde(default, skip_serializing_if = "Fx::is_default")]
    pub fx: Fx,
}

/// Bandes de l'égaliseur : (filtre `filter-chain`, fréquence Hz, Q, libellé).
pub const EQ_BANDS: [(&str, f32, f32, &str); 5] = [
    ("bq_lowshelf", 80.0, 0.7, "80"),
    ("bq_peaking", 250.0, 1.0, "250"),
    ("bq_peaking", 1000.0, 1.0, "1k"),
    ("bq_peaking", 4000.0, 1.0, "4k"),
    ("bq_highshelf", 10000.0, 0.7, "10k"),
];
/// Plage des gains de l'égaliseur (± dB).
pub const EQ_RANGE: f32 = 12.0;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Fx {
    /// Égaliseur 5 bandes actif.
    #[serde(default)]
    pub eq: bool,
    /// Gains des bandes en dB (`EQ_BANDS`).
    #[serde(default)]
    pub gains: [f32; 5],
    /// Suppression de bruit RNNoise (plugin LADSPA du système).
    #[serde(default)]
    pub denoise: bool,
}

impl Fx {
    pub fn is_default(&self) -> bool {
        *self == Fx::default()
    }
}

pub fn stereo() -> Vec<String> {
    vec!["FL".into(), "FR".into()]
}

/// Dispositions proposées dans l'interface.
pub const LAYOUTS: &[(&str, &[&str])] = &[
    ("Mono", &["MONO"]),
    ("Stereo", &["FL", "FR"]),
    ("Quad", &["FL", "FR", "RL", "RR"]),
    ("5.1", &["FL", "FR", "FC", "LFE", "RL", "RR"]),
    ("7.1", &["FL", "FR", "FC", "LFE", "RL", "RR", "SL", "SR"]),
];

pub fn layout_name(channels: &[String]) -> String {
    LAYOUTS
        .iter()
        .find(|(_, l)| l.len() == channels.len() && l.iter().zip(channels).all(|(a, b)| a == b))
        .map(|(n, _)| n.to_string())
        .unwrap_or_else(|| format!("{} ch", channels.len()))
}

/// Convertit un gain en dB vers l'amplitude linéaire appliquée par PipeWire.
pub fn db_to_linear(db: f32) -> f32 {
    if db <= MIN_DB { 0.0 } else { 10f32.powf(db / 20.0) }
}

pub fn linear_to_db(lin: f32) -> f32 {
    if lin <= 0.0 { MIN_DB } else { (20.0 * lin.log10()).clamp(MIN_DB, MAX_DB) }
}

/// Un périphérique audio du système (hors VeeMeeter).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Device {
    pub name: String,
    pub description: String,
    /// `true` = source (micro), `false` = sink (sortie).
    pub capture: bool,
    pub channels: Vec<String>,
}

/// Un flux d'application (lecture ou enregistrement).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct App {
    pub id: u32,
    pub name: String,
    pub media: String,
    pub icon: Option<String>,
    /// `true` = l'application joue du son, `false` = elle enregistre.
    pub playback: bool,
    /// Nœud auquel le flux est actuellement relié.
    pub target: Option<String>,
    /// Strip VeeMeeter correspondant à `target`, s'il y en a un.
    pub target_strip: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StripState {
    #[serde(flatten)]
    pub strip: Strip,
    /// Le nœud PipeWire du strip existe.
    pub ready: bool,
    /// Le périphérique physique est branché (strips hardware).
    pub online: bool,
    /// Le strip est le périphérique par défaut actuel du système.
    pub is_default: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub strips: Vec<StripState>,
    pub devices: Vec<Device>,
    pub apps: Vec<App>,
    pub default_sink: Option<String>,
    pub default_source: Option<String>,
    pub pipewire_version: String,
    /// Le plugin RNNoise est installé : la suppression de bruit est possible.
    #[serde(default)]
    pub denoise_available: bool,
}

impl Snapshot {
    pub fn strip(&self, id: &str) -> Option<&StripState> {
        self.strips.iter().find(|s| s.strip.id == id)
    }

    /// Libellé court d'une sortie : `A1`, `A2`… / `B1`, `B2`… selon l'ordre.
    pub fn bus_label(&self, id: &str) -> String {
        bus_label(self.strips.iter().map(|s| &s.strip), id)
    }
}

pub fn bus_label<'a>(strips: impl Iterator<Item = &'a Strip>, id: &str) -> String {
    let (mut a, mut b) = (0, 0);
    for s in strips {
        match s.kind {
            Kind::HardwareOut => a += 1,
            Kind::VirtualOut => b += 1,
            _ => continue,
        }
        if s.id == id {
            return if s.kind == Kind::HardwareOut { format!("A{a}") } else { format!("B{b}") };
        }
    }
    id.to_uppercase()
}

/// Requêtes envoyées au daemon (une par ligne JSON).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    /// Abonnement : l'état est poussé à chaque changement, les niveaux
    /// (VU-mètres) seulement si `meters`.
    Subscribe {
        meters: bool,
    },
    Add {
        kind: Kind,
        name: String,
        device: Option<String>,
        channels: Option<Vec<String>>,
    },
    Remove {
        id: String,
    },
    Rename {
        id: String,
        name: String,
    },
    SetDevice {
        id: String,
        device: Option<String>,
    },
    SetChannels {
        id: String,
        channels: Vec<String>,
    },
    Volume {
        id: String,
        db: f32,
    },
    /// `None` = bascule.
    Mute {
        id: String,
        mute: Option<bool>,
    },
    /// `None` = bascule.
    Route {
        from: String,
        to: String,
        on: Option<bool>,
    },
    Primary {
        id: String,
    },
    /// Égaliseur : `enabled` (`None` = inchangé) et/ou nouveaux gains.
    Eq {
        id: String,
        enabled: Option<bool>,
        gains: Option<[f32; 5]>,
    },
    /// Suppression de bruit, `None` = bascule.
    Denoise {
        id: String,
        on: Option<bool>,
    },
    /// Déplace un flux d'application vers un strip (ou un nœud par son nom).
    MoveApp {
        app: u32,
        to: String,
    },
    /// Déplace un strip dans l'ordre d'affichage.
    Reorder {
        id: String,
        delta: i32,
    },
    /// Répondu par `Message::Pong` une fois les commandes précédentes traitées.
    Ping {
        tag: u64,
    },
    Quit,
}

/// Messages du daemon (une ligne JSON chacun).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "msg", rename_all = "snake_case")]
pub enum Message {
    State {
        state: Snapshot,
    },
    /// Crêtes par canal (amplitude 0..1+) depuis la trame précédente.
    Meters {
        levels: Vec<(String, Vec<f32>)>,
    },
    Error {
        text: String,
    },
    Pong {
        tag: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_roundtrip() {
        for db in [-40.0f32, -6.0, 0.0, 6.0] {
            assert!((linear_to_db(db_to_linear(db)) - db).abs() < 1e-3);
        }
        assert_eq!(db_to_linear(MIN_DB), 0.0);
        assert_eq!(linear_to_db(0.0), MIN_DB);
    }

    #[test]
    fn bus_labels_follow_order() {
        let mk = |id: &str, kind| Strip {
            id: id.into(),
            kind,
            name: id.into(),
            device: None,
            channels: stereo(),
            volume_db: 0.0,
            mute: false,
            routes: vec![],
            primary: false,
            fx: Fx::default(),
        };
        let s = [
            mk("hi1", Kind::HardwareIn),
            mk("a3", Kind::HardwareOut),
            mk("b1", Kind::VirtualOut),
            mk("a1", Kind::HardwareOut),
        ];
        assert_eq!(bus_label(s.iter(), "a3"), "A1");
        assert_eq!(bus_label(s.iter(), "a1"), "A2");
        assert_eq!(bus_label(s.iter(), "b1"), "B1");
    }

    #[test]
    fn commands_are_tagged_json() {
        let c: Command = serde_json::from_str(r#"{"cmd":"route","from":"vi1","to":"a1","on":true}"#).unwrap();
        assert_eq!(c, Command::Route { from: "vi1".into(), to: "a1".into(), on: Some(true) });
    }
}

//! Effets d'un strip (égaliseur 5 bandes, suppression de bruit RNNoise).
//!
//! Chaque strip qui a des effets reçoit une instance du module PipeWire
//! `filter-chain`, chargée **dans le process du daemon** : elle crée deux
//! nœuds, `veemeeter.fx.<id>.in` (entrée) et `veemeeter.fx.<id>.out`
//! (sortie), que le moteur relie comme les autres. Les gains se règlent en
//! direct (paramètre `Props.params`), sans recharger le module ; seuls
//! l'activation d'un effet ou un changement de canaux le rechargent.

use crate::model::{EQ_BANDS, Fx, Strip};
use pipewire::{self as pw, spa};
use spa::pod::{Object, Property, PropertyFlags, Value, serialize::PodSerializer};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

const RNNOISE_LIB: &str = "librnnoise_ladspa.so";

pub fn in_name(id: &str) -> String {
    format!("veemeeter.fx.{id}.in")
}

pub fn out_name(id: &str) -> String {
    format!("veemeeter.fx.{id}.out")
}

/// Ce qui oblige à recharger le module quand ça change.
#[derive(Clone, PartialEq, Debug)]
pub struct FxSig {
    pub channels: Vec<String>,
    pub eq: bool,
    pub denoise: bool,
}

/// Effets réellement actifs pour un strip (`None` = aucun module).
pub fn wanted(s: &Strip, rnnoise: Option<&Path>) -> Option<FxSig> {
    let denoise = s.fx.denoise && rnnoise.is_some();
    (s.fx.eq || denoise).then(|| FxSig { channels: s.channels.clone(), eq: s.fx.eq, denoise })
}

/// Cherche le plugin LADSPA de RNNoise (paquets `noise-suppression-for-voice`,
/// `rnnoise-plugin`, `ladspa-rnnoise`…).
pub fn find_rnnoise() -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> =
        std::env::var_os("LADSPA_PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let user = std::env::var("USER").unwrap_or_default();
    for d in [
        "/usr/lib/ladspa",
        "/usr/lib64/ladspa",
        "/usr/local/lib/ladspa",
        "/usr/lib/x86_64-linux-gnu/ladspa",
        "/usr/lib/aarch64-linux-gnu/ladspa",
        "/run/current-system/sw/lib/ladspa",
    ] {
        dirs.push(d.into());
    }
    dirs.push(format!("/etc/profiles/per-user/{user}/lib/ladspa").into());
    if let Some(h) = home {
        dirs.push(h.join(".nix-profile/lib/ladspa"));
        dirs.push(h.join(".ladspa"));
    }
    dirs.into_iter().map(|d| d.join(RNNOISE_LIB)).find(|p| p.is_file())
}

/// Arguments SPA-JSON du module `filter-chain` (le JSON est du SPA-JSON valide).
pub fn module_args(s: &Strip, sig: &FxSig, rnnoise: Option<&Path>) -> String {
    let q = |v: &str| serde_json::to_string(v).expect("string serializes");
    let mut nodes = vec![];
    if sig.denoise {
        if let Some(p) = rnnoise {
            nodes.push((
                "rnn".to_string(),
                format!(
                    "{{ type = ladspa name = rnn plugin = {} label = noise_suppressor_mono }}",
                    q(&p.to_string_lossy())
                ),
                ("Input", "Output"),
            ));
        }
    }
    if sig.eq {
        for (i, (label, freq, qf, _)) in EQ_BANDS.iter().enumerate() {
            let name = format!("eq{}", i + 1);
            nodes.push((
                name.clone(),
                format!(
                    "{{ type = builtin name = {name} label = {label} control = {{ \"Freq\" = {freq:.1} \"Q\" = {qf:.2} \"Gain\" = {:.1} }} }}",
                    s.fx.gains[i]
                ),
                ("In", "Out"),
            ));
        }
    }
    let links: Vec<String> = nodes
        .windows(2)
        .map(|w| format!("{{ output = \"{}:{}\" input = \"{}:{}\" }}", w[0].0, w[0].2.1, w[1].0, w[1].2.0))
        .collect();
    let stream = |name: String| {
        format!(
            "{{ node.name = {} node.autoconnect = false node.dont-reconnect = true stream.dont-remix = true }}",
            q(&name)
        )
    };
    format!(
        "{{ node.description = {} media.name = {} audio.channels = {} audio.position = [ {} ] \
         filter.graph = {{ nodes = [ {} ] links = [ {} ] }} capture.props = {} playback.props = {} }}",
        q(&format!("VeeMeeter FX {}", s.name)),
        q(&format!("VeeMeeter FX {}", s.name)),
        sig.channels.len(),
        sig.channels.join(" "),
        nodes.iter().map(|n| n.1.as_str()).collect::<Vec<_>>().join(" "),
        links.join(" "),
        stream(in_name(&s.id)),
        stream(out_name(&s.id)),
    )
}

/// `Props { params = [ "eq1:Gain" g1 … ] }` pour régler les gains en direct.
pub fn gains_pod(fx: &Fx) -> Vec<u8> {
    let mut items = vec![];
    for (i, g) in fx.gains.iter().enumerate() {
        items.push(Value::String(format!("eq{}:Gain", i + 1)));
        items.push(Value::Float(*g));
    }
    let value = Value::Object(Object {
        type_: spa::sys::SPA_TYPE_OBJECT_Props,
        id: spa::sys::SPA_PARAM_Props,
        properties: vec![Property {
            key: spa::sys::SPA_PROP_params,
            flags: PropertyFlags::empty(),
            value: Value::Struct(items),
        }],
    });
    PodSerializer::serialize(std::io::Cursor::new(Vec::new()), &value).expect("pod serializes").0.into_inner()
}

/// Instance du module chargée dans notre contexte ; déchargée au drop.
pub struct Module(NonNull<pw::sys::pw_impl_module>);

impl Module {
    pub fn load(context: &pw::context::Context, args: &str) -> Result<Module, String> {
        let name = CString::new("libpipewire-module-filter-chain").expect("no NUL");
        let args = CString::new(args).map_err(|_| "NUL in module arguments")?;
        // SAFETY : appelé depuis le thread de la boucle principale, contexte vivant.
        let ptr = unsafe {
            pw::sys::pw_context_load_module(context.as_raw_ptr(), name.as_ptr(), args.as_ptr(), std::ptr::null_mut())
        };
        NonNull::new(ptr).map(Module).ok_or_else(|| "filter-chain refused the effect graph".to_string())
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY : pointeur obtenu de pw_context_load_module, détruit une seule fois.
        unsafe { pw::sys::pw_impl_module_destroy(self.0.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Kind, stereo};

    fn strip() -> Strip {
        Strip {
            id: "hi1".into(),
            kind: Kind::HardwareIn,
            name: "Mic \"pro\"".into(),
            device: None,
            channels: stereo(),
            volume_db: 0.0,
            mute: false,
            routes: vec![],
            sends: Default::default(),
            primary: false,
            fx: Fx { eq: true, gains: [3.0, 0.0, 0.0, -2.5, 0.0], denoise: true },
        }
    }

    #[test]
    fn wanted_needs_the_plugin_for_denoise() {
        let mut s = strip();
        s.fx.eq = false;
        assert_eq!(wanted(&s, None), None);
        assert!(wanted(&s, Some(Path::new("/x.so"))).unwrap().denoise);
        s.fx = Fx::default();
        assert_eq!(wanted(&s, Some(Path::new("/x.so"))), None);
    }

    #[test]
    fn args_chain_denoise_then_eq() {
        let s = strip();
        let p = Path::new("/lib/ladspa/librnnoise_ladspa.so");
        let sig = wanted(&s, Some(p)).unwrap();
        let a = module_args(&s, &sig, Some(p));
        assert!(a.contains(r#"plugin = "/lib/ladspa/librnnoise_ladspa.so" label = noise_suppressor_mono"#));
        assert!(a.contains(r#"{ output = "rnn:Output" input = "eq1:In" }"#));
        assert!(a.contains(r#"{ output = "eq4:Out" input = "eq5:In" }"#));
        assert!(a.contains(r#""Gain" = -2.5"#));
        assert!(a.contains("audio.channels = 2 audio.position = [ FL FR ]"));
        assert!(a.contains(r#"node.name = "veemeeter.fx.hi1.in""#));
        // Les guillemets du nom sont échappés.
        assert!(a.contains(r#"VeeMeeter FX Mic \"pro\""#));
    }

    #[test]
    fn eq_only_has_no_ladspa() {
        let mut s = strip();
        s.fx.denoise = false;
        let sig = wanted(&s, None).unwrap();
        let a = module_args(&s, &sig, None);
        assert!(!a.contains("ladspa"));
        assert_eq!(a.matches("type = builtin").count(), 5);
    }
}

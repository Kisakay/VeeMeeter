//! Paramètres SPA : volume/mute (`Props`) et format des flux de mesure.

use pipewire::spa;
use spa::pod::{
    Object, Pod, Property, PropertyFlags, Value, ValueArray, deserialize::PodDeserializer, serialize::PodSerializer,
};
use spa::sys;

fn serialize(value: &Value) -> Vec<u8> {
    PodSerializer::serialize(std::io::Cursor::new(Vec::new()), value).expect("pod serializes").0.into_inner()
}

/// `Props { channelVolumes, mute }` — linéaire, un gain par canal.
pub fn props(volumes: &[f32], mute: bool) -> Vec<u8> {
    serialize(&Value::Object(Object {
        type_: sys::SPA_TYPE_OBJECT_Props,
        id: sys::SPA_PARAM_Props,
        properties: vec![
            Property {
                key: sys::SPA_PROP_channelVolumes,
                flags: PropertyFlags::empty(),
                value: Value::ValueArray(ValueArray::Float(volumes.to_vec())),
            },
            Property { key: sys::SPA_PROP_mute, flags: PropertyFlags::empty(), value: Value::Bool(mute) },
        ],
    }))
}

/// Lit `channelVolumes` et `mute` d'un pod `Props` (les deux doivent y être).
pub fn parse_props(pod: &Pod) -> Option<(Vec<f32>, bool)> {
    let (_, value) = PodDeserializer::deserialize_any_from(pod.as_bytes()).ok()?;
    let Value::Object(obj) = value else { return None };
    let (mut vols, mut mute) = (None, None);
    for p in obj.properties {
        match (p.key, p.value) {
            (k, Value::ValueArray(ValueArray::Float(v))) if k == sys::SPA_PROP_channelVolumes => vols = Some(v),
            (k, Value::Bool(b)) if k == sys::SPA_PROP_mute => mute = Some(b),
            _ => {}
        }
    }
    Some((vols?, mute?))
}

/// Format F32 entrelacé, débit du graphe, positions données.
pub fn audio_format(channels: &[String]) -> Vec<u8> {
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_channels(channels.len() as u32);
    let mut pos = [0u32; sys::SPA_AUDIO_MAX_CHANNELS as usize];
    for (slot, ch) in pos.iter_mut().zip(channels) {
        *slot = channel_id(ch);
    }
    info.set_position(pos);
    serialize(&Value::Object(Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: info.into(),
    }))
}

pub fn channel_id(name: &str) -> u32 {
    use sys::*;
    match name {
        "MONO" => SPA_AUDIO_CHANNEL_MONO,
        "FL" => SPA_AUDIO_CHANNEL_FL,
        "FR" => SPA_AUDIO_CHANNEL_FR,
        "FC" => SPA_AUDIO_CHANNEL_FC,
        "LFE" => SPA_AUDIO_CHANNEL_LFE,
        "SL" => SPA_AUDIO_CHANNEL_SL,
        "SR" => SPA_AUDIO_CHANNEL_SR,
        "FLC" => SPA_AUDIO_CHANNEL_FLC,
        "FRC" => SPA_AUDIO_CHANNEL_FRC,
        "RC" => SPA_AUDIO_CHANNEL_RC,
        "RL" => SPA_AUDIO_CHANNEL_RL,
        "RR" => SPA_AUDIO_CHANNEL_RR,
        "TC" => SPA_AUDIO_CHANNEL_TC,
        "TFL" => SPA_AUDIO_CHANNEL_TFL,
        "TFC" => SPA_AUDIO_CHANNEL_TFC,
        "TFR" => SPA_AUDIO_CHANNEL_TFR,
        "TRL" => SPA_AUDIO_CHANNEL_TRL,
        "TRC" => SPA_AUDIO_CHANNEL_TRC,
        "TRR" => SPA_AUDIO_CHANNEL_TRR,
        other => match other.strip_prefix("AUX").and_then(|n| n.parse::<u32>().ok()) {
            Some(n) if n < 64 => SPA_AUDIO_CHANNEL_AUX0 + n,
            _ => SPA_AUDIO_CHANNEL_UNKNOWN,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_roundtrip() {
        let bytes = props(&[0.5, 0.25], true);
        let pod = Pod::from_bytes(&bytes).unwrap();
        assert_eq!(parse_props(pod), Some((vec![0.5, 0.25], true)));
    }

    #[test]
    fn channel_ids() {
        assert_eq!(channel_id("FL"), sys::SPA_AUDIO_CHANNEL_FL);
        assert_eq!(channel_id("AUX3"), sys::SPA_AUDIO_CHANNEL_AUX0 + 3);
        assert_eq!(channel_id("??"), sys::SPA_AUDIO_CHANNEL_UNKNOWN);
    }
}

//! VU-mètres : un flux de capture par strip, relié par le moteur aux ports
//! de sortie du nœud du strip (liens passifs : la mesure ne réveille jamais
//! le graphe). Le thread temps réel ne fait que des `fetch_max` atomiques.

use super::pod;
use pipewire::{self as pw, properties::properties, spa};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct Meter {
    /// Gardé vivant : le relâcher déconnecte et détruit le flux.
    _stream: pw::stream::StreamRc,
    _listener: pw::stream::StreamListener<Arc<[AtomicU32]>>,
    pub levels: Arc<[AtomicU32]>,
    pub node_name: String,
    pub channels: Vec<String>,
}

impl Meter {
    pub fn new(core: &pw::core::CoreRc, strip_id: &str, channels: &[String]) -> Result<Meter, pw::Error> {
        let node_name = format!("veemeeter.meter.{strip_id}");
        let props = properties! {
            "media.type" => "Audio",
            "media.category" => "Capture",
            "media.role" => "Production",
            "media.name" => "VeeMeeter level meter",
            "node.name" => node_name.as_str(),
            "node.description" => format!("VeeMeeter meter {strip_id}"),
            "node.autoconnect" => "false",
            "node.passive" => "true",
            "node.dont-reconnect" => "true",
            "stream.dont-remix" => "true",
            "stream.monitor" => "true",
            "application.name" => "VeeMeeter",
            "application.id" => "veemeeter",
        };
        let stream = pw::stream::StreamRc::new(core.clone(), &node_name, props)?;
        let levels: Arc<[AtomicU32]> = (0..channels.len()).map(|_| AtomicU32::new(0)).collect();
        let n = channels.len();
        let listener = stream
            .add_local_listener_with_user_data(levels.clone())
            .process(move |stream, levels| {
                let Some(mut buffer) = stream.dequeue_buffer() else { return };
                let Some(data) = buffer.datas_mut().first_mut() else { return };
                let size = data.chunk().size() as usize;
                let offset = data.chunk().offset() as usize;
                let Some(bytes) = data.data() else { return };
                let Some(bytes) = bytes.get(offset..offset + size) else { return };
                let mut peaks = [0f32; 32];
                for (i, s) in bytes.chunks_exact(4).enumerate() {
                    let v = f32::from_le_bytes([s[0], s[1], s[2], s[3]]).abs();
                    let c = i % n;
                    if c < 32 && v > peaks[c] {
                        peaks[c] = v;
                    }
                }
                for (slot, p) in levels.iter().zip(peaks) {
                    if p.is_finite() {
                        // f32 positifs : l'ordre des bits suit l'ordre des valeurs.
                        slot.fetch_max(p.to_bits(), Ordering::Relaxed);
                    }
                }
            })
            .register()?;
        let format = pod::audio_format(channels);
        let mut params = [spa::pod::Pod::from_bytes(&format).expect("valid pod")];
        stream.connect(
            spa::utils::Direction::Input,
            None,
            pw::stream::StreamFlags::MAP_BUFFERS | pw::stream::StreamFlags::RT_PROCESS,
            &mut params,
        )?;
        Ok(Meter { _stream: stream, _listener: listener, levels, node_name, channels: channels.to_vec() })
    }

    /// Crêtes depuis le dernier appel (remises à zéro).
    pub fn take(&self) -> Vec<f32> {
        self.levels.iter().map(|l| f32::from_bits(l.swap(0, Ordering::Relaxed))).collect()
    }
}

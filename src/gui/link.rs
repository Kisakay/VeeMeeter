//! Lien avec le daemon, commun à la fenêtre et au tray (aucun toolkit ici) :
//! état partagé, thread de connexion qui relance le daemon au besoin, et
//! échelle / retombée des VU-mètres.

use crate::ipc::{self, Client};
use crate::model::{Command, Message, Snapshot};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Échelle des VU-mètres (dBFS).
pub const METER_FLOOR: f32 = -60.0;
pub const METER_TOP: f32 = 6.0;

#[derive(Default)]
pub struct Shared {
    pub sender: Option<ipc::Sender>,
    pub state: Option<Snapshot>,
    pub meters: HashMap<String, Vec<f32>>,
    pub meters_at: Option<Instant>,
    pub error: Option<(String, Instant)>,
    pub connected: bool,
    pub status: String,
}

impl Shared {
    pub fn new() -> Arc<Mutex<Shared>> {
        Arc::new(Mutex::new(Shared { status: "connecting…".into(), ..Default::default() }))
    }

    /// Envoie une commande au daemon ; la connexion perdue est oubliée.
    pub fn send(&mut self, cmd: &Command) {
        if let Some(tx) = self.sender.as_mut()
            && tx.send(cmd).is_err()
        {
            self.sender = None;
        }
    }
}

/// Thread de connexion : se (re)connecte au daemon, le démarre au besoin,
/// range chaque message dans l'état partagé puis appelle `notify`.
pub fn start_connection(shared: Arc<Mutex<Shared>>, notify: impl Fn() + Send + 'static, spawn: bool) {
    std::thread::Builder::new()
        .name("daemon-link".into())
        .spawn(move || {
            let mut spawned_at: Option<Instant> = None;
            loop {
                match Client::connect() {
                    Ok(mut client) => {
                        if client.send(&Command::Subscribe { meters: true }).is_err() {
                            continue;
                        }
                        let (sender, rx) = client.split();
                        {
                            let mut s = shared.lock().unwrap();
                            s.sender = Some(sender);
                            s.connected = true;
                            s.status = "connected".into();
                        }
                        for msg in rx {
                            let mut s = shared.lock().unwrap();
                            match msg {
                                Ok(Message::State { state }) => s.state = Some(state),
                                Ok(Message::Meters { levels }) => {
                                    s.meters = levels.into_iter().collect();
                                    s.meters_at = Some(Instant::now());
                                }
                                Ok(Message::Error { text }) => s.error = Some((text, Instant::now())),
                                Ok(Message::Pong { .. }) => {}
                                Err(_) => break,
                            }
                            drop(s);
                            notify();
                        }
                        let mut s = shared.lock().unwrap();
                        s.sender = None;
                        s.connected = false;
                        s.status = "daemon stopped — reconnecting…".into();
                        drop(s);
                        notify();
                    }
                    Err(_) => {
                        let due = spawned_at.is_none_or(|t| t.elapsed() > Duration::from_secs(10));
                        if spawn && due {
                            spawned_at = Some(Instant::now());
                            let msg = match spawn_daemon() {
                                Ok(()) => "starting the daemon…".to_string(),
                                Err(e) => format!("cannot start the daemon: {e}"),
                            };
                            shared.lock().unwrap().status = msg;
                            notify();
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(400));
            }
        })
        .expect("spawn connection thread");
}

/// Lance le daemon s'il ne tourne pas : le service systemd s'il est
/// installé, sinon `veemeeter daemon` détaché. Il survit à la fenêtre : le
/// routage continue quand on la ferme.
fn spawn_daemon() -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command as Proc, Stdio};
    let systemd = Proc::new("systemctl")
        .args(["--user", "start", "veemeeter.service"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if matches!(systemd, Ok(s) if s.success()) {
        return Ok(());
    }
    Proc::new(std::env::current_exe()?)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map(|_| ())
}

/// Amplitude crête → position 0..1 sur l'échelle du VU-mètre.
pub fn meter_pos(amp: f32) -> f32 {
    if amp <= 0.0 { 0.0 } else { meter_pos_db(20.0 * amp.log10()) }
}

pub fn meter_pos_db(db: f32) -> f32 {
    ((db - METER_FLOOR) / (METER_TOP - METER_FLOOR)).clamp(0.0, 1.0)
}

/// Un canal de VU-mètre affiché : niveau qui retombe, crête qui tient.
#[derive(Clone, Copy, Debug)]
pub struct MeterView {
    pub level: f32,
    pub peak: f32,
    pub peak_at: Instant,
}

/// Balistique des VU-mètres, par `(strip, canal)`.
#[derive(Default)]
pub struct Meters {
    pub views: HashMap<(String, usize), MeterView>,
}

impl Meters {
    /// Fait retomber les niveaux de `dt` secondes puis applique les crêtes reçues.
    pub fn update(&mut self, levels: &HashMap<String, Vec<f32>>, dt: f32) {
        let now = Instant::now();
        for m in self.views.values_mut() {
            // Retombée ≈ 26 dB/s (échelle normalisée sur 66 dB).
            m.level = (m.level - dt * 0.4).max(0.0);
            if now.duration_since(m.peak_at) > Duration::from_millis(1200) {
                m.peak = (m.peak - dt * 0.6).max(m.level);
            }
        }
        for (id, chans) in levels {
            for (i, &amp) in chans.iter().enumerate() {
                let v = meter_pos(amp);
                let m = self.views.entry((id.clone(), i)).or_insert(MeterView { level: 0.0, peak: 0.0, peak_at: now });
                if v > m.level {
                    m.level = v;
                }
                if v >= m.peak {
                    m.peak = v;
                    m.peak_at = now;
                }
            }
        }
    }

    pub fn get(&self, id: &str, channel: usize) -> Option<&MeterView> {
        self.views.get(&(id.to_string(), channel))
    }

    /// Un canal bouge encore (il faut continuer d'animer).
    pub fn animating(&self) -> bool {
        self.views.values().any(|m| m.level > 0.001 || m.peak > 0.001)
    }

    /// Le plus fort niveau affiché (humeur de Mimi).
    pub fn loudest(&self) -> f32 {
        self.views.values().map(|m| m.level).fold(0.0, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_scale() {
        assert_eq!(meter_pos(0.0), 0.0);
        assert!((meter_pos_db(METER_TOP) - 1.0).abs() < 1e-6);
        assert!(meter_pos(0.5) > meter_pos(0.25));
    }

    #[test]
    fn meters_fall_back() {
        let mut m = Meters::default();
        m.update(&HashMap::from([("vi1".to_string(), vec![1.0])]), 0.0);
        let top = m.get("vi1", 0).unwrap().level;
        m.update(&HashMap::new(), 0.5);
        assert!(m.get("vi1", 0).unwrap().level < top);
        assert!(m.animating());
    }
}

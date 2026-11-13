//! Icône de zone de notification (StatusNotifierItem via D-Bus) : Mimi danse
//! au rythme de ce qui sort des haut-parleurs. Processus à part
//! (`veemeeter tray`) : sous Wayland une fenêtre ne peut pas être cachée, le
//! tray ouvre et ferme donc la fenêtre comme un processus séparé.

use super::instance::{self, Order};
use super::mascot::{Mood, decode_rgba};
use super::settings::Settings;
use super::{Shared, meter_pos, meter_pos_db, start_connection};
use ksni::blocking::TrayMethods;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const REPO_URL: &str = "https://github.com/Kisakay/VeeMeeter";
pub const ISSUE_URL: &str = "https://github.com/Kisakay/VeeMeeter/issues/new";

/// Côté de l'icône envoyée (le panneau la réduit à sa taille).
const ICON: usize = 64;
/// Cadence d'animation ; rien n'est envoyé quand l'image ne change pas.
const FRAME: Duration = Duration::from_millis(66);
/// Après ce silence, Mimi s'endort.
const NAP_AFTER: Duration = Duration::from_secs(30);

/// Lance le tray. `Err` : pas de tray possible (rôle déjà pris, ou aucun
/// StatusNotifierWatcher sur le bureau) ; l'appelant peut ouvrir la fenêtre.
pub fn run(spawn_daemon: bool) -> Result<i32, String> {
    let lock = instance::claim("tray").ok_or("the tray is already running")?;
    instance::listen(lock, |_| {});

    let shared = Arc::new(Mutex::new(Shared { status: "connecting…".into(), ..Default::default() }));
    let sprites = Sprites::load();
    let quit = Arc::new(AtomicBool::new(false));
    let tray = VeeTray {
        icon: sprites.frame(Mood::Sleepy, 0.0, 0.0, 0.0),
        tooltip: "Waking the audio engine up…".into(),
        start_on_tray: Settings::load().start_on_tray,
        spawn_daemon,
        children: Vec::new(),
        quit: quit.clone(),
    };
    let handle = tray.spawn().map_err(|e| format!("no system tray available: {e}"))?;
    start_connection(shared.clone(), || {}, spawn_daemon);

    let mut anim = Anim::new();
    let mut last = Instant::now();
    let mut shown = (Vec::new(), String::new());
    let mut reaped = Instant::now();
    while !quit.load(Ordering::Relaxed) && !handle.is_closed() {
        std::thread::sleep(FRAME);
        let dt = last.elapsed().as_secs_f32().min(0.25);
        last = Instant::now();
        let (connected, error, level) = read_levels(&shared);
        let (mood, tooltip) = anim.step(dt, connected, error, level);
        let icon = sprites.frame(mood, anim.hop, anim.squash, anim.env);
        // Chaque `update` repasse sur toutes les propriétés D-Bus : seulement
        // quand l'image ou la bulle change.
        if icon.data != shown.0 || tooltip != shown.1 {
            shown = (icon.data.clone(), tooltip.clone());
            handle.update(|t| {
                t.icon = icon;
                t.tooltip = tooltip;
            });
        }
        if reaped.elapsed() > Duration::from_secs(2) {
            reaped = Instant::now();
            handle.update(|t| t.children.retain_mut(|c| matches!(c.try_wait(), Ok(None))));
        }
    }
    handle.shutdown().wait();
    Ok(0)
}

/// État de la connexion et niveau (0..1) le plus fort sur les sorties.
fn read_levels(shared: &Mutex<Shared>) -> (bool, bool, f32) {
    let s = shared.lock().unwrap();
    let error = s.error.as_ref().is_some_and(|(_, at)| at.elapsed() < Duration::from_secs(3));
    let fresh = s.meters_at.is_some_and(|t| t.elapsed() < Duration::from_millis(300));
    let mut level: f32 = 0.0;
    if fresh {
        for (id, chans) in &s.meters {
            let is_output = s.state.as_ref().and_then(|st| st.strip(id)).is_none_or(|x| !x.strip.kind.is_input());
            let muted = s.state.as_ref().and_then(|st| st.strip(id)).is_some_and(|x| x.strip.mute);
            if is_output && !muted {
                level = chans.iter().map(|&a| meter_pos(a)).fold(level, f32::max);
            }
        }
    }
    (s.connected, error, level)
}

/// Petite physique de danse : une enveloppe qui suit le niveau, un rebond
/// déclenché sur les attaques, et un écrasement à la retombée.
struct Anim {
    /// Niveau lissé (attaque immédiate, retombée lente).
    env: f32,
    /// Moyenne lente : sert à repérer les attaques (temps forts).
    slow: f32,
    /// Hauteur du saut (0..1) et vitesse verticale.
    hop: f32,
    vel: f32,
    /// Écrasement à l'atterrissage (0..1).
    squash: f32,
    quiet_since: Instant,
}

impl Anim {
    fn new() -> Anim {
        Anim { env: 0.0, slow: 0.0, hop: 0.0, vel: 0.0, squash: 0.0, quiet_since: Instant::now() }
    }

    fn step(&mut self, dt: f32, connected: bool, error: bool, level: f32) -> (Mood, String) {
        self.env = level.max(self.env - dt * 0.9);
        self.slow += (self.env - self.slow) * (dt * 2.5).min(1.0);
        // Temps fort : le niveau dépasse nettement sa moyenne → Mimi saute,
        // d'autant plus haut que la musique est forte.
        if self.hop <= 0.0 && level > 0.2 && level - self.slow > 0.06 {
            self.vel = 2.2 + 3.5 * level;
        }
        // Musique continue sans attaque nette : petit trot régulier.
        if self.hop <= 0.0 && self.vel <= 0.0 && self.env > 0.35 {
            self.vel = 1.2 + 1.5 * self.env;
        }
        self.vel -= 22.0 * dt;
        self.hop += self.vel * dt;
        if self.hop <= 0.0 {
            if self.vel < -1.0 {
                self.squash = (-self.vel / 6.0).min(1.0);
            }
            self.hop = 0.0;
            self.vel = 0.0;
        }
        self.squash = (self.squash - dt * 5.0).max(0.0);

        let playing = self.env > 0.05;
        if playing {
            self.quiet_since = Instant::now();
        }
        let mood = if error {
            Mood::Surprised
        } else if !connected || self.quiet_since.elapsed() > NAP_AFTER {
            Mood::Sleepy
        } else if self.env > meter_pos_db(-12.0) {
            Mood::Happy
        } else {
            Mood::Wave
        };
        let tooltip = if !connected {
            "Audio engine stopped".to_string()
        } else if playing {
            "Mimi is dancing".to_string()
        } else {
            "All quiet".to_string()
        };
        (mood, tooltip)
    }
}

/// Mimi pré-décodée (RGBA prémultiplié, recadrée), une image par humeur.
struct Sprites {
    size: usize,
    moods: Vec<(Mood, Vec<[f32; 4]>)>,
}

impl Sprites {
    fn load() -> Sprites {
        let decoded: Vec<(Mood, usize, Vec<u8>)> = Mood::ALL
            .iter()
            .map(|&m| {
                let ([w, _], rgba) = decode_rgba(m.png());
                (m, w, rgba)
            })
            .collect();
        // Les PNG ont de la marge : recadrage carré sur la silhouette (union des
        // humeurs, pour que Mimi ne change pas de taille en changeant de tête).
        let full = decoded[0].1;
        let (mut x0, mut y0, mut x1, mut y1) = (full, full, 0, 0);
        for (_, w, rgba) in &decoded {
            for (i, p) in rgba.chunks_exact(4).enumerate() {
                if p[3] > 8 {
                    let (x, y) = (i % w, i / w);
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                }
            }
        }
        let side = (x1.saturating_sub(x0)).max(y1.saturating_sub(y0)).max(1);
        // Centrée horizontalement, pieds en bas.
        let ox = (x0 + x1) as isize / 2 - side as isize / 2;
        let oy = y1 as isize - side as isize;
        let moods = decoded
            .into_iter()
            .map(|(m, w, rgba)| {
                let mut px = vec![[0.0f32; 4]; side * side];
                for (y, row) in px.chunks_exact_mut(side).enumerate() {
                    for (x, out) in row.iter_mut().enumerate() {
                        let (sx, sy) = (x as isize + ox, y as isize + oy);
                        if sx < 0 || sy < 0 || sx as usize >= w || sy as usize >= w {
                            continue;
                        }
                        let p = &rgba[(sy as usize * w + sx as usize) * 4..][..4];
                        let a = p[3] as f32 / 255.0;
                        *out = [p[0] as f32 * a, p[1] as f32 * a, p[2] as f32 * a, a];
                    }
                }
                (m, px)
            })
            .collect();
        Sprites { size: side, moods }
    }

    /// Image ARGB32 (ordre réseau) : Mimi posée au bas de l'icône, soulevée
    /// de `hop`, écrasée de `squash`, avec un égaliseur quand ça joue.
    fn frame(&self, mood: Mood, hop: f32, squash: f32, env: f32) -> ksni::Icon {
        let src = &self.moods.iter().find(|(m, _)| *m == mood).expect("every mood is loaded").1;
        let n = ICON as f32;
        let base = n * 0.86;
        let w = base * (1.0 + 0.12 * squash);
        let h = base * (1.0 - 0.12 * squash);
        let x0 = (n - w) / 2.0;
        // Quantifié au pixel : moins d'images différentes → moins de trafic D-Bus.
        let y0 = (n - h - hop.min(1.0) * n * 0.12).round();
        let mut out = vec![[0.0f32; 4]; ICON * ICON];
        let s = self.size as f32;
        for py in 0..ICON {
            let v0 = (py as f32 - y0) / h * s;
            let v1 = (py as f32 + 1.0 - y0) / h * s;
            if v1 <= 0.0 || v0 >= s {
                continue;
            }
            for px in 0..ICON {
                let u0 = (px as f32 - x0) / w * s;
                let u1 = (px as f32 + 1.0 - x0) / w * s;
                if u1 <= 0.0 || u0 >= s {
                    continue;
                }
                out[py * ICON + px] = box_average(src, self.size, u0, u1, v0, v1);
            }
        }
        if env > 0.05 {
            draw_eq(&mut out, env, hop);
        }
        let mut data = Vec::with_capacity(ICON * ICON * 4);
        for [r, g, b, a] in out {
            let un = |c: f32| if a > 0.0 { (c / a).clamp(0.0, 255.0) as u8 } else { 0 };
            data.extend_from_slice(&[(a * 255.0).round() as u8, un(r), un(g), un(b)]);
        }
        ksni::Icon { width: ICON as i32, height: ICON as i32, data }
    }
}

/// Moyenne (prémultipliée) des pixels source couverts par le pixel cible.
fn box_average(src: &[[f32; 4]], size: usize, u0: f32, u1: f32, v0: f32, v1: f32) -> [f32; 4] {
    let (xa, xb) = (u0.max(0.0) as usize, (u1.ceil() as usize).clamp(1, size));
    let (ya, yb) = (v0.max(0.0) as usize, (v1.ceil() as usize).clamp(1, size));
    let mut acc = [0.0f32; 4];
    let mut count = 0.0;
    for y in ya..yb {
        for x in xa..xb {
            let p = src[y * size + x];
            for i in 0..4 {
                acc[i] += p[i];
            }
            count += 1.0;
        }
    }
    // Pixels hors de l'image (bord) comptés transparents.
    let total = (u1 - u0) * (v1 - v0);
    let k = if total > 0.0 { 1.0 / total.max(count) } else { 0.0 };
    acc.map(|c| c * k)
}

/// Trois barres d'égaliseur dans le coin, qui suivent le volume.
fn draw_eq(out: &mut [[f32; 4]], env: f32, hop: f32) {
    // Couleurs du thème : menthe, rose, lavande.
    const COLORS: [[f32; 3]; 3] = [[127.0, 224.0, 194.0], [255.0, 143.0, 184.0], [185.0, 163.0, 245.0]];
    let wobble = [0.75, 1.0, 0.6];
    for (i, color) in COLORS.iter().enumerate() {
        let phase = if i == 1 { 1.0 - hop.min(1.0) * 0.5 } else { 0.7 + hop.min(1.0) * 0.3 };
        let height = ((env * wobble[i] * phase) * 22.0).round().max(2.0) as usize;
        let x = ICON - 15 + i * 5;
        for y in ICON - height..ICON {
            for dx in 0..4 {
                let p = &mut out[y * ICON + x + dx];
                // Contour sombre (couleur `on_accent`) sur la colonne de gauche.
                let c = if dx == 0 { [43.0, 33.0, 64.0] } else { *color };
                *p = [c[0], c[1], c[2], 1.0];
            }
        }
    }
}

struct VeeTray {
    icon: ksni::Icon,
    tooltip: String,
    start_on_tray: bool,
    spawn_daemon: bool,
    /// Fenêtres lancées par le tray (récoltées pour ne pas laisser de zombies).
    children: Vec<Child>,
    quit: Arc<AtomicBool>,
}

impl VeeTray {
    fn open_window(&mut self) {
        if instance::tell_gui(Order::Show) {
            return;
        }
        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap_or_else(|_| "veemeeter".into()));
        cmd.arg("gui");
        if !self.spawn_daemon {
            cmd.arg("--no-spawn");
        }
        match cmd.spawn() {
            Ok(c) => self.children.push(c),
            Err(e) => log::error!("cannot open the window: {e}"),
        }
    }
}

impl ksni::Tray for VeeTray {
    fn id(&self) -> String {
        "veemeeter".into()
    }

    fn title(&self) -> String {
        "VeeMeeter".into()
    }

    fn icon_name(&self) -> String {
        // Repli pour les panneaux qui ignorent les pixmaps.
        "veemeeter".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![self.icon.clone()]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip { title: "VeeMeeter".into(), description: self.tooltip.clone(), ..Default::default() }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.open_window();
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Open Vee".into(),
                icon_name: "window-new".into(),
                activate: Box::new(|t: &mut Self| t.open_window()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            CheckmarkItem {
                label: "Start on tray".into(),
                checked: self.start_on_tray,
                activate: Box::new(|t: &mut Self| {
                    t.start_on_tray = !t.start_on_tray;
                    Settings::set_start_on_tray(t.start_on_tray);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Open GitHub".into(),
                icon_name: "applications-internet".into(),
                activate: Box::new(|_: &mut Self| open_url(REPO_URL)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Report an issue".into(),
                icon_name: "dialog-warning".into(),
                activate: Box::new(|_: &mut Self| open_url(ISSUE_URL)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Close Vee".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|t: &mut Self| {
                    instance::tell_gui(Order::Quit);
                    t.quit.store(true, Ordering::Relaxed);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn watcher_offline(&self, reason: ksni::OfflineReason) -> bool {
        log::warn!("system tray went away: {reason:?}");
        true
    }
}

/// Ouvre une page dans le navigateur par défaut, sans bloquer.
pub fn open_url(url: &str) {
    use std::os::unix::process::CommandExt;
    let r = std::process::Command::new("xdg-open")
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn();
    match r {
        // Récolté en arrière-plan : xdg-open rend la main tout de suite.
        Ok(mut c) => drop(std::thread::spawn(move || c.wait())),
        Err(e) => log::error!("cannot open {url}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_argb_and_move_with_the_music() {
        let s = Sprites::load();
        let still = s.frame(Mood::Wave, 0.0, 0.0, 0.0);
        assert_eq!(still.data.len(), ICON * ICON * 4);
        // Coins du haut transparents, Mimi bien présente.
        assert_eq!(still.data[0], 0);
        assert!(still.data.chunks_exact(4).filter(|p| p[0] > 200).count() > ICON * ICON / 4);
        let jump = s.frame(Mood::Happy, 1.0, 0.0, 0.8);
        assert_ne!(still.data, jump.data);
    }

    #[test]
    fn mimi_jumps_on_beats_and_naps_in_silence() {
        let mut a = Anim::new();
        let (mood, _) = a.step(0.066, true, false, 0.0);
        assert_eq!(mood, Mood::Wave);
        a.step(0.066, true, false, 0.9);
        a.step(0.066, true, false, 0.9);
        assert!(a.hop > 0.0, "a loud attack makes Mimi jump");
        assert_eq!(a.step(0.066, true, false, 0.9).0, Mood::Happy);
        assert_eq!(a.step(0.066, false, false, 0.0).0, Mood::Sleepy);
        assert_eq!(a.step(0.066, true, true, 0.0).0, Mood::Surprised);
    }
}

//! Fenêtre de mixage (egui, rendu OpenGL natif). Simple client du daemon :
//! tout l'état vient du socket, chaque geste devient une `Command`.

mod instance;
mod mascot;
mod settings;
mod theme;
pub mod tray;

use crate::ipc::{self, Client};
use crate::model::*;
use eframe::egui::{self, Align, CornerRadius, Layout, RichText, Sense, Stroke, Ui, vec2};
use mascot::{Mimi, Mood};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use theme::THEME as T;

const STRIP_W: f32 = 138.0;
const FADER_H: f32 = 210.0;
/// Échelle des VU-mètres (dBFS).
const METER_FLOOR: f32 = -60.0;
const METER_TOP: f32 = 6.0;

#[derive(Default)]
struct Shared {
    sender: Option<ipc::Sender>,
    state: Option<Snapshot>,
    meters: HashMap<String, Vec<f32>>,
    meters_at: Option<Instant>,
    error: Option<(String, Instant)>,
    connected: bool,
    status: String,
}

/// Point d'entrée de `veemeeter` / `veemeeter gui` : ramène la fenêtre si
/// elle est déjà ouverte, sinon démarre dans le tray (option « Start on
/// tray ») ou ouvre la fenêtre, avec le tray à côté.
pub fn launch(spawn: bool, with_tray: bool) -> i32 {
    if instance::tell_gui(instance::Order::Show) {
        return 0;
    }
    if with_tray && settings::Settings::load().start_on_tray && !instance::running("tray") {
        match tray::run(spawn) {
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
    let shared = Arc::new(Mutex::new(Shared { status: "connecting…".into(), ..Default::default() }));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("VeeMeeter")
            .with_app_id("veemeeter")
            .with_inner_size([1600.0, 740.0])
            .with_min_inner_size([520.0, 420.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        "VeeMeeter",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            let ctx = cc.egui_ctx.clone();
            start_connection(shared.clone(), move || ctx.request_repaint(), spawn);
            let ctx = cc.egui_ctx.clone();
            instance::listen(listener, move |order| {
                match order {
                    instance::Order::Show => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    instance::Order::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                }
                ctx.request_repaint();
            });
            let mimi = Mimi::load(&cc.egui_ctx);
            Ok(Box::new(App::new(shared, mimi)))
        }),
    );
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("veemeeter: cannot open the window: {e}");
            1
        }
    }
}

/// Thread de connexion : se (re)connecte au daemon, le démarre au besoin,
/// et range chaque message dans l'état partagé.
fn start_connection(shared: Arc<Mutex<Shared>>, notify: impl Fn() + Send + 'static, spawn: bool) {
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

struct MeterView {
    level: f32,
    peak: f32,
    peak_at: Instant,
}

struct App {
    shared: Arc<Mutex<Shared>>,
    mimi: Mimi,
    /// Valeur du fader pendant un glissé (l'écho du daemon arrive après).
    dragging: HashMap<String, f32>,
    /// Niveaux d'envoi pendant un glissé.
    send_drag: HashMap<(String, String), f32>,
    /// Gains d'égaliseur pendant un glissé.
    eq_drag: HashMap<String, [f32; 5]>,
    meters: HashMap<(String, usize), MeterView>,
    rename: HashMap<String, String>,
    show_apps: bool,
    settings: settings::Settings,
    last_frame: Instant,
}

impl App {
    fn new(shared: Arc<Mutex<Shared>>, mimi: Mimi) -> App {
        App {
            shared,
            mimi,
            dragging: HashMap::new(),
            eq_drag: HashMap::new(),
            send_drag: HashMap::new(),
            meters: HashMap::new(),
            rename: HashMap::new(),
            show_apps: true,
            settings: settings::Settings::load(),
            last_frame: Instant::now(),
        }
    }

    fn send(&self, cmd: Command) {
        let mut s = self.shared.lock().unwrap();
        if let Some(tx) = s.sender.as_mut() {
            if tx.send(&cmd).is_err() {
                s.sender = None;
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let (state, levels, connected, status, error) = {
            let mut s = self.shared.lock().unwrap();
            if s.error.as_ref().is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(6)) {
                s.error = None;
            }
            let fresh = s.meters_at.is_some_and(|t| t.elapsed() < Duration::from_millis(300));
            (
                s.state.clone(),
                if fresh { s.meters.clone() } else { HashMap::new() },
                s.connected,
                s.status.clone(),
                s.error.as_ref().map(|e| e.0.clone()),
            )
        };
        let dt = self.last_frame.elapsed().as_secs_f32().min(0.2);
        self.last_frame = Instant::now();
        self.update_meters(&levels, dt);

        let ctx = ui.ctx().clone();
        egui::Panel::top("top")
            .frame(egui::Frame::new().fill(T.bg).inner_margin(egui::Margin::symmetric(14, 10)))
            .show(ui, |ui| self.top_bar(ui, state.as_ref(), connected, &status, error.as_deref()));

        if let Some(st) = &state {
            if self.show_apps {
                egui::Panel::bottom("apps")
                    .resizable(true)
                    .default_size(170.0)
                    .frame(egui::Frame::new().fill(T.bg).inner_margin(egui::Margin::symmetric(14, 10)))
                    .show(ui, |ui| self.apps_panel(ui, st));
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(T.panel).inner_margin(egui::Margin::same(14)))
            .show(ui, |ui| match &state {
                Some(st) => self.mixer(ui, st),
                None => {
                    ui.vertical_centered(|ui| {
                        ui.add_space((ui.available_height() / 2.0 - 140.0).max(0.0));
                        ui.add(self.mimi.image(Mood::Sleepy, 192.0));
                        ui.label(RichText::new("Mimi is waking the audio engine up…").size(18.0).color(T.text));
                        ui.label(RichText::new(&status).color(T.muted));
                    });
                }
            });

        // Les VU-mètres retombent en douceur même sans nouvelle trame.
        if self.meters.values().any(|m| m.level > 0.001 || m.peak > 0.001) {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
    }
}

impl App {
    fn update_meters(&mut self, levels: &HashMap<String, Vec<f32>>, dt: f32) {
        let now = Instant::now();
        for m in self.meters.values_mut() {
            // Retombée ≈ 26 dB/s (échelle normalisée sur 66 dB).
            m.level = (m.level - dt * 0.4).max(0.0);
            if now.duration_since(m.peak_at) > Duration::from_millis(1200) {
                m.peak = (m.peak - dt * 0.6).max(m.level);
            }
        }
        for (id, chans) in levels {
            for (i, &amp) in chans.iter().enumerate() {
                let v = meter_pos(amp);
                let m = self.meters.entry((id.clone(), i)).or_insert(MeterView { level: 0.0, peak: 0.0, peak_at: now });
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

    fn top_bar(&mut self, ui: &mut Ui, st: Option<&Snapshot>, connected: bool, status: &str, error: Option<&str>) {
        ui.horizontal(|ui| {
            // Mimi réagit : coucou au repos, joyeuse quand ça pousse, surprise sur une erreur.
            let loud = self.meters.values().any(|m| m.level > meter_pos_db(-12.0));
            let mood = if error.is_some() {
                Mood::Surprised
            } else if !connected {
                Mood::Sleepy
            } else if loud {
                Mood::Happy
            } else {
                Mood::Wave
            };
            ui.add(self.mimi.image(mood, 40.0)).on_hover_text("Mimi, VeeMeeter's sound sprite");
            ui.label(RichText::new("Vee").size(24.0).strong().color(T.accent));
            ui.add_space(-6.0);
            ui.label(RichText::new("Meeter").size(24.0).strong().color(T.accent2));
            ui.add_space(16.0);
            ui.add_enabled_ui(connected, |ui| {
                ui.menu_button(RichText::new("+  Add").strong().color(T.accent), |ui| {
                    for kind in [Kind::HardwareIn, Kind::VirtualIn, Kind::Cable, Kind::HardwareOut, Kind::VirtualOut] {
                        if ui.button(kind.label()).clicked() {
                            self.add_strip(kind, st);
                            ui.close();
                        }
                    }
                });
                ui.toggle_value(&mut self.show_apps, "Applications");
            });
            egui::containers::menu::MenuButton::new(RichText::new("Vee").color(T.muted))
                .config(sticky_menu())
                .ui(ui, |ui| self.vee_menu(ui));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (dot, text) = match (connected, st) {
                    (true, Some(s)) => (T.meter_low, format!("PipeWire {}", s.pipewire_version)),
                    _ => (T.meter_mid, status.to_string()),
                };
                ui.label(RichText::new(text).color(T.muted));
                icon(ui, Icon::Dot, dot);
                if connected
                    && ui
                        .button(RichText::new("Stop engine").color(T.muted))
                        .on_hover_text("Stops the daemon: every VeeMeeter device disappears until it is started again")
                        .clicked()
                {
                    self.send(Command::Quit);
                }
                if let Some(e) = error {
                    ui.label(RichText::new(format!("⚠ {e}")).color(T.mute));
                }
            });
        });
    }

    /// Menu « Vee » : préférences de l'interface et liens du projet.
    fn vee_menu(&mut self, ui: &mut Ui) {
        ui.set_min_width(220.0);
        // Relu à l'ouverture : le tray a pu changer l'option entre-temps.
        if ui.ctx().input(|i| i.pointer.any_pressed()) {
            self.settings = settings::Settings::load();
        }
        let mut on = self.settings.start_on_tray;
        if ui
            .checkbox(&mut on, RichText::new("Start on tray").color(T.text))
            .on_hover_text("Next launch shows only Mimi in the system tray; click her to open the mixer")
            .changed()
        {
            self.settings.start_on_tray = on;
            settings::Settings::set_start_on_tray(on);
        }
        ui.separator();
        if ui.button("Open GitHub").clicked() {
            tray::open_url(tray::REPO_URL);
            ui.close();
        }
        if ui.button("Report an issue").clicked() {
            tray::open_url(tray::ISSUE_URL);
            ui.close();
        }
    }

    fn add_strip(&self, kind: Kind, st: Option<&Snapshot>) {
        let n = st.map(|s| s.strips.iter().filter(|x| x.strip.kind == kind).count() + 1).unwrap_or(1);
        let name = match kind {
            Kind::HardwareIn => format!("Input {n}"),
            Kind::VirtualIn => format!("Virtual {n}"),
            Kind::Cable => format!("Cable {n}"),
            Kind::HardwareOut => format!("Output {n}"),
            Kind::VirtualOut => format!("Virtual Mic {n}"),
        };
        // Strips matériels : premier périphérique pas encore utilisé.
        let device = st.filter(|_| kind.is_hardware()).and_then(|s| {
            s.devices
                .iter()
                .filter(|d| d.capture == (kind == Kind::HardwareIn))
                .find(|d| !s.strips.iter().any(|x| x.strip.device.as_deref() == Some(&d.name)))
                .map(|d| d.name.clone())
        });
        self.send(Command::Add { kind, name, device, channels: None });
    }

    fn mixer(&mut self, ui: &mut Ui, st: &Snapshot) {
        egui::ScrollArea::horizontal().auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let in_group = |k: Kind, g: usize| match g {
                    0 => matches!(k, Kind::HardwareIn | Kind::VirtualIn),
                    1 => k == Kind::Cable,
                    _ => !k.is_input(),
                };
                let has_cables = st.strips.iter().any(|s| s.strip.kind == Kind::Cable);
                for (g, title) in [(0, "INPUTS"), (1, "CABLES"), (2, "OUTPUTS")] {
                    if g == 1 && !has_cables {
                        continue;
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(title).size(13.0).strong().color(T.muted));
                        ui.horizontal_top(|ui| {
                            let strips: Vec<&StripState> =
                                st.strips.iter().filter(|s| in_group(s.strip.kind, g)).collect();
                            if strips.is_empty() {
                                ui.label(RichText::new("Nothing here yet, use + Add").color(T.muted));
                            }
                            for s in strips {
                                self.strip_card(ui, st, s);
                            }
                        });
                    });
                    if g < 2 {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);
                    }
                }
            });
        });
    }

    fn strip_card(&mut self, ui: &mut Ui, st: &Snapshot, s: &StripState) {
        let x = &s.strip;
        let kind_color = match x.kind {
            Kind::Cable => T.accent3,
            k if k.is_hardware() => T.hw,
            _ => T.virt,
        };
        egui::Frame::new()
            .fill(T.card)
            .corner_radius(CornerRadius::same(14))
            .stroke(Stroke::new(1.5, if s.is_default { T.star } else { T.border }))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(STRIP_W);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let tag = if x.kind.is_input() { x.id.to_uppercase() } else { st.bus_label(&x.id) };
                        ui.label(
                            RichText::new(format!(" {tag} "))
                                .small()
                                .strong()
                                .color(T.on_accent)
                                .background_color(kind_color),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // Ces popups contiennent des champs et des curseurs : elles ne se
                            // ferment qu'au clic à l'extérieur (un clic dans le champ de
                            // renommage fermait le menu).
                            egui::containers::menu::MenuButton::new(RichText::new("•••").color(T.muted))
                                .config(sticky_menu())
                                .ui(ui, |ui| self.strip_menu(ui, st, s));
                            let active = x.fx.eq || (x.fx.denoise && st.denoise_available);
                            let label =
                                RichText::new("FX").small().strong().color(if active { T.on_accent } else { T.muted });
                            let fx_button = egui::Button::new(label).fill(if active { T.accent3 } else { T.card_hi });
                            egui::containers::menu::MenuButton::from_button(fx_button)
                                .config(sticky_menu())
                                .ui(ui, |ui| self.fx_menu(ui, st, s))
                                .0
                                .on_hover_text("Equalizer and noise suppression");
                        });
                    });
                    ui.add(egui::Label::new(RichText::new(&x.name).size(15.0).strong().color(T.text)).truncate());
                    let sub = format!(
                        "{} · {}",
                        match x.kind {
                            Kind::Cable => "cable",
                            k if k.is_hardware() => "hardware",
                            _ => "virtual",
                        },
                        layout_name(&x.channels)
                    );
                    ui.label(RichText::new(sub).small().color(T.muted));

                    if x.kind.is_hardware() {
                        self.device_picker(ui, st, s);
                    } else {
                        ui.add_space(24.0);
                    }
                    let small = ui.text_style_height(&egui::TextStyle::Small);
                    if !s.ready {
                        ui.label(RichText::new("starting…").small().color(T.meter_mid));
                    } else if x.kind.is_hardware() && !s.online {
                        ui.label(RichText::new("device unplugged").small().color(T.mute));
                    } else {
                        ui.add_space(small);
                    }

                    ui.horizontal(|ui| {
                        self.meter(ui, &x.id, x.channels.len(), x.mute);
                        ui.add_space(4.0);
                        self.fader(ui, x);
                    });
                    let db = self.dragging.get(&x.id).copied().unwrap_or(x.volume_db);
                    let text = if db <= MIN_DB { "−∞ dB".to_string() } else { format!("{db:+.1} dB") };
                    ui.vertical_centered(|ui| ui.label(RichText::new(text).monospace().color(T.muted)));

                    ui.horizontal(|ui| {
                        let mute = ui.add(
                            egui::Button::new(RichText::new("MUTE").strong().color(if x.mute {
                                T.on_accent
                            } else {
                                T.text
                            }))
                            .fill(if x.mute { T.mute } else { T.card_hi })
                            .min_size(vec2(60.0, 26.0)),
                        );
                        if mute.clicked() {
                            self.send(Command::Mute { id: x.id.clone(), mute: Some(!x.mute) });
                        }
                        if !x.kind.is_hardware() && x.kind != Kind::Cable {
                            let star = ui.add(
                                egui::Button::new(
                                    RichText::new(if s.is_default { "DEFAULT" } else { "default" })
                                        .small()
                                        .strong()
                                        .color(if s.is_default { T.on_accent } else { T.star }),
                                )
                                .fill(if s.is_default { T.star } else { T.card_hi })
                                .min_size(vec2(58.0, 26.0)),
                            );
                            let star = star.on_hover_text(if x.kind.is_input() {
                                "Make this the system default output (apps play here)"
                            } else {
                                "Make this the system default microphone"
                            });
                            if star.clicked() {
                                self.send(Command::Primary { id: x.id.clone() });
                            }
                        }
                    });

                    if x.kind.is_input() {
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            for out in st.strips.iter().filter(|o| !o.strip.kind.is_input()) {
                                let on = x.routes.contains(&out.strip.id);
                                let c = if out.strip.kind.is_hardware() { T.hw } else { T.virt };
                                let send = x.sends.get(&out.strip.id).copied();
                                let label = match send {
                                    Some(db) if db <= MIN_DB => format!("{} −∞", st.bus_label(&out.strip.id)),
                                    Some(db) => format!("{} {db:+.0}", st.bus_label(&out.strip.id)),
                                    None => st.bus_label(&out.strip.id),
                                };
                                let b = ui
                                    .add(
                                        egui::Button::new(RichText::new(label).strong().color(if on {
                                            T.on_accent
                                        } else {
                                            c
                                        }))
                                        .fill(if on { c } else { T.card_hi })
                                        .min_size(vec2(36.0, 24.0)),
                                    )
                                    .on_hover_text(format!(
                                        "Send to {} — right-click for the send level",
                                        out.strip.name
                                    ));
                                egui::Popup::context_menu(&b)
                                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                    .show(|ui| self.send_menu(ui, x, &out.strip));
                                if b.clicked() {
                                    self.send(Command::Route {
                                        from: x.id.clone(),
                                        to: out.strip.id.clone(),
                                        on: Some(!on),
                                    });
                                }
                            }
                        });
                    }
                });
            });
    }

    fn device_picker(&mut self, ui: &mut Ui, st: &Snapshot, s: &StripState) {
        let x = &s.strip;
        let capture = x.kind == Kind::HardwareIn;
        let current = match x.device.as_deref() {
            Some(d) => st
                .devices
                .iter()
                .find(|dv| dv.name == d)
                .map(|dv| dv.description.clone())
                .unwrap_or_else(|| "(unplugged)".into()),
            None => "No device".into(),
        };
        egui::ComboBox::from_id_salt(("dev", &x.id)).width(STRIP_W - 4.0).truncate().selected_text(current).show_ui(
            ui,
            |ui| {
                if ui.selectable_label(x.device.is_none(), "No device").clicked() {
                    self.send(Command::SetDevice { id: x.id.clone(), device: None });
                }
                for d in st.devices.iter().filter(|d| d.capture == capture) {
                    let sel = x.device.as_deref() == Some(&d.name);
                    if ui.selectable_label(sel, &d.description).on_hover_text(&d.name).clicked() {
                        self.send(Command::SetDevice { id: x.id.clone(), device: Some(d.name.clone()) });
                    }
                }
            },
        );
    }

    /// Niveau d'envoi d'une entrée vers une sortie (clic droit sur la route).
    fn send_menu(&mut self, ui: &mut Ui, from: &Strip, to: &Strip) {
        ui.set_min_width(230.0);
        ui.label(RichText::new(format!("{} → {}", from.name, to.name)).strong().color(T.text));
        let key = (from.id.clone(), to.id.clone());
        let current = from.sends.get(&to.id).copied().unwrap_or(0.0);
        let mut db = self.send_drag.get(&key).copied().unwrap_or(current);
        let before = db;
        ui.spacing_mut().slider_width = 200.0;
        let r = ui.add(egui::Slider::new(&mut db, MIN_DB..=MAX_DB).suffix(" dB").fixed_decimals(1));
        if r.dragged() {
            self.send_drag.insert(key, db);
        } else {
            self.send_drag.remove(&key);
        }
        if ui.button("Back to 0 dB").clicked() {
            db = 0.0;
        }
        ui.label(RichText::new("Only this route: the input's fader still applies.").small().color(T.muted));
        if (db - before).abs() > 0.05 {
            self.send(Command::Send { from: from.id.clone(), to: to.id.clone(), db });
        }
    }

    fn fx_menu(&mut self, ui: &mut Ui, st: &Snapshot, s: &StripState) {
        let x = &s.strip;
        ui.set_min_width(260.0);
        let mut eq = x.fx.eq;
        if ui.checkbox(&mut eq, RichText::new("Equalizer").strong().color(T.text)).changed() {
            self.send(Command::Eq { id: x.id.clone(), enabled: Some(eq), gains: None });
        }
        let mut gains = self.eq_drag.get(&x.id).copied().unwrap_or(x.fx.gains);
        let before = gains;
        let mut dragging = false;
        ui.add_enabled_ui(x.fx.eq, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().slider_width = 120.0;
                for (i, (_, _, _, label)) in EQ_BANDS.iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.set_width(42.0);
                        let r = ui.add(
                            egui::Slider::new(&mut gains[i], -EQ_RANGE..=EQ_RANGE)
                                .vertical()
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        if r.double_clicked() {
                            gains[i] = 0.0;
                        }
                        dragging |= r.dragged();
                        ui.label(RichText::new(format!("{:+.0}", gains[i])).small().monospace().color(T.muted));
                        ui.label(RichText::new(*label).small().strong().color(T.text));
                    });
                }
            });
            if ui.button("Flat").on_hover_text("All bands back to 0 dB").clicked() {
                gains = [0.0; 5];
            }
        });
        let gains = gains.map(|g| (g * 2.0).round() / 2.0);
        if dragging {
            self.eq_drag.insert(x.id.clone(), gains);
        } else {
            self.eq_drag.remove(&x.id);
        }
        if gains != before.map(|g| (g * 2.0).round() / 2.0) {
            self.send(Command::Eq { id: x.id.clone(), enabled: None, gains: Some(gains) });
        }
        ui.separator();
        let mut dn = x.fx.denoise;
        let r = ui.add_enabled(
            st.denoise_available,
            egui::Checkbox::new(&mut dn, RichText::new("Noise suppression").strong().color(T.text)),
        );
        let r = if st.denoise_available {
            r.on_hover_text("RNNoise: removes background noise (fans, keyboard, hum) from a voice")
        } else {
            r.on_disabled_hover_text("Install the RNNoise LADSPA plugin (librnnoise_ladspa.so) to enable it")
        };
        if r.changed() {
            self.send(Command::Denoise { id: x.id.clone(), on: Some(dn) });
        }
    }

    fn strip_menu(&mut self, ui: &mut Ui, st: &Snapshot, s: &StripState) {
        let x = &s.strip;
        ui.set_min_width(200.0);
        ui.label(RichText::new("Name").small().color(T.muted));
        let mut buf = self.rename.remove(&x.id).unwrap_or_else(|| x.name.clone());
        let r = ui.text_edit_singleline(&mut buf);
        if r.lost_focus() && buf.trim() != x.name && !buf.trim().is_empty() {
            self.send(Command::Rename { id: x.id.clone(), name: buf.trim().to_string() });
        }
        if r.has_focus() {
            self.rename.insert(x.id.clone(), buf); // brouillon gardé pendant la saisie
        }
        ui.separator();
        ui.label(RichText::new("Channels").small().color(T.muted));
        ui.horizontal_wrapped(|ui| {
            for (name, layout) in LAYOUTS {
                let sel = layout.len() == x.channels.len() && layout.iter().zip(&x.channels).all(|(a, b)| a == b);
                if ui.selectable_label(sel, *name).clicked() && !sel {
                    self.send(Command::SetChannels {
                        id: x.id.clone(),
                        channels: layout.iter().map(|c| c.to_string()).collect(),
                    });
                }
            }
        });
        if let Some(dev) = x.device.as_deref().and_then(|d| st.devices.iter().find(|dv| dv.name == d)) {
            if !dev.channels.is_empty()
                && dev.channels != x.channels
                && ui.button(format!("Match device ({})", layout_name(&dev.channels))).clicked()
            {
                self.send(Command::SetChannels { id: x.id.clone(), channels: dev.channels.clone() });
            }
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Move left").clicked() {
                self.send(Command::Reorder { id: x.id.clone(), delta: -1 });
            }
            if ui.button("Move right").clicked() {
                self.send(Command::Reorder { id: x.id.clone(), delta: 1 });
            }
        });
        if ui.button("Reset volume (0 dB)").clicked() {
            self.send(Command::Volume { id: x.id.clone(), db: 0.0 });
        }
        ui.separator();
        if ui.button(RichText::new("Remove").color(T.mute)).clicked() {
            self.send(Command::Remove { id: x.id.clone() });
            ui.close();
        }
    }

    fn fader(&mut self, ui: &mut Ui, x: &Strip) {
        let before = self.dragging.get(&x.id).copied().unwrap_or(x.volume_db);
        let mut db = before;
        ui.spacing_mut().slider_width = FADER_H;
        let r = ui
            .add(egui::Slider::new(&mut db, MIN_DB..=MAX_DB).vertical().show_value(false).trailing_fill(true))
            .on_hover_text("Drag, scroll, or double-click for 0 dB");
        if r.double_clicked() {
            db = 0.0;
        } else if r.hovered() && !r.dragged() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                db = (db + scroll / 40.0).clamp(MIN_DB, MAX_DB);
            }
        }
        let db = (db * 10.0).round() / 10.0;
        if r.dragged() {
            self.dragging.insert(x.id.clone(), db);
        } else {
            self.dragging.remove(&x.id);
        }
        if (db - before).abs() > 0.05 || (r.drag_stopped() && (db - x.volume_db).abs() > 0.05) {
            self.send(Command::Volume { id: x.id.clone(), db });
        }
    }

    fn meter(&mut self, ui: &mut Ui, id: &str, channels: usize, muted: bool) {
        let bars = channels.clamp(1, 8);
        let bar_w = if bars > 2 { 6.0 } else { 9.0 };
        let (rect, _) = ui.allocate_exact_size(vec2(bars as f32 * (bar_w + 3.0) + 3.0, FADER_H), Sense::hover());
        let p = ui.painter_at(rect);
        let y_of = |r: egui::Rect, v: f32| r.bottom() - v.clamp(0.0, 1.0) * r.height();
        for c in 0..bars {
            let left = rect.left() + c as f32 * (bar_w + 3.0);
            let bar = egui::Rect::from_min_max(egui::pos2(left, rect.top()), egui::pos2(left + bar_w, rect.bottom()));
            p.rect_filled(bar, CornerRadius::same(3), T.meter_bg);
            let Some(m) = self.meters.get(&(id.to_string(), c)) else { continue };
            // Segments pleins, pas de dégradé : vert / jaune (≥ −12 dB) / rouge (≥ −3 dB).
            let segs = [
                (0.0, meter_pos_db(-12.0), T.meter_low),
                (meter_pos_db(-12.0), meter_pos_db(-3.0), T.meter_mid),
                (meter_pos_db(-3.0), 1.0, T.meter_high),
            ];
            for (lo, hi, color) in segs {
                let top = m.level.min(hi);
                if top > lo {
                    let color = if muted { color.gamma_multiply(0.35) } else { color };
                    let seg = egui::Rect::from_x_y_ranges(bar.x_range(), y_of(bar, top)..=y_of(bar, lo));
                    p.rect_filled(seg, CornerRadius::same(2), color);
                }
            }
            if m.peak > 0.01 {
                let py = y_of(bar, m.peak);
                p.line_segment([egui::pos2(bar.left(), py), egui::pos2(bar.right(), py)], Stroke::new(2.0, T.text));
            }
        }
        for db in [0.0, -12.0, -24.0, -48.0] {
            let yy = y_of(rect, meter_pos_db(db));
            p.line_segment(
                [egui::pos2(rect.right() - 2.0, yy), egui::pos2(rect.right(), yy)],
                Stroke::new(1.0, T.muted),
            );
        }
    }

    fn apps_panel(&mut self, ui: &mut Ui, st: &Snapshot) {
        ui.label(RichText::new("APPLICATIONS").size(13.0).strong().color(T.muted));
        if st.apps.is_empty() {
            ui.label(RichText::new("No application is playing or recording right now").color(T.muted));
            return;
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("apps").num_columns(4).spacing(vec2(14.0, 6.0)).show(ui, |ui| {
                for a in &st.apps {
                    icon(
                        ui,
                        if a.playback { Icon::Play } else { Icon::Dot },
                        if a.playback { T.accent2 } else { T.accent },
                    )
                    .on_hover_text(if a.playback { "playing" } else { "recording" });
                    fixed(ui, 170.0, egui::Label::new(RichText::new(&a.name).strong().color(T.text)).truncate());
                    fixed(ui, 260.0, egui::Label::new(RichText::new(&a.media).color(T.muted)).truncate());
                    let current = a
                        .target_strip
                        .as_deref()
                        .and_then(|id| st.strip(id))
                        .map(|s| s.strip.name.clone())
                        .or_else(|| {
                            a.target.as_deref().map(|t| {
                                st.devices
                                    .iter()
                                    .find(|d| d.name == t)
                                    .map(|d| d.description.clone())
                                    .unwrap_or_else(|| t.to_string())
                            })
                        })
                        .unwrap_or_else(|| "—".into());
                    egui::ComboBox::from_id_salt(("app", a.id)).width(240.0).truncate().selected_text(current).show_ui(
                        ui,
                        |ui| {
                            let kind = if a.playback { Kind::VirtualIn } else { Kind::VirtualOut };
                            ui.label(RichText::new("VeeMeeter").small().color(T.muted));
                            // Un câble se joue ET s'enregistre : proposé dans les deux sens.
                            for s in st.strips.iter().filter(|s| s.strip.kind == kind || s.strip.kind == Kind::Cable) {
                                let sel = a.target_strip.as_deref() == Some(&s.strip.id);
                                if ui.selectable_label(sel, &s.strip.name).clicked() {
                                    self.send(Command::MoveApp { app: a.id, to: s.strip.id.clone() });
                                }
                            }
                            ui.label(RichText::new("Devices").small().color(T.muted));
                            for d in st.devices.iter().filter(|d| d.capture != a.playback) {
                                let sel = a.target.as_deref() == Some(&d.name);
                                if ui.selectable_label(sel, &d.description).clicked() {
                                    self.send(Command::MoveApp { app: a.id, to: d.name.clone() });
                                }
                            }
                        },
                    );
                    ui.end_row();
                }
            });
        });
    }
}

/// Amplitude crête → position 0..1 sur l'échelle du VU-mètre.
pub(crate) fn meter_pos(amp: f32) -> f32 {
    if amp <= 0.0 { 0.0 } else { meter_pos_db(20.0 * amp.log10()) }
}

pub(crate) fn meter_pos_db(db: f32) -> f32 {
    ((db - METER_FLOOR) / (METER_TOP - METER_FLOOR)).clamp(0.0, 1.0)
}

#[derive(Clone, Copy)]
enum Icon {
    Dot,
    Play,
}

/// Petites icônes dessinées (les polices embarquées n'ont pas ces glyphes).
fn icon(ui: &mut Ui, icon: Icon, color: egui::Color32) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    match icon {
        Icon::Dot => p.circle_filled(c, 4.5, color),
        Icon::Play => p.add(egui::Shape::convex_polygon(
            vec![c + vec2(-4.0, -5.0), c + vec2(5.0, 0.0), c + vec2(-4.0, 5.0)],
            color,
            Stroke::NONE,
        )),
    };
    r
}

/// Widget aligné à gauche dans une colonne de largeur fixe.
fn fixed(ui: &mut Ui, width: f32, widget: impl egui::Widget) -> egui::Response {
    ui.allocate_ui_with_layout(vec2(width, 18.0), Layout::left_to_right(Align::Center), |ui| {
        ui.set_width(width);
        ui.add(widget)
    })
    .inner
}

/// Menu qui reste ouvert tant qu'on clique dedans (champs, curseurs).
fn sticky_menu() -> egui::containers::menu::MenuConfig {
    egui::containers::menu::MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
}

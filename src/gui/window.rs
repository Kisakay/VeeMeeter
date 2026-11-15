//! Fenêtre de mixage en libcosmic (toolkit de COSMIC, sur iced) : barre de
//! titre et thème du système, couleurs de marque pour les VU-mètres, les
//! étiquettes et les routes. Simple client du daemon : l'état vient du
//! socket (`link`), chaque geste devient une `Command`.

use super::instance::{self, Order};
use super::link::{self, Meters, Shared, meter_pos_db};
use super::mascot::{Mood, decode_rgba};
use super::{settings, theme as T, tray};
use crate::model::*;
use cosmic::app::{Core, Task};
use cosmic::iced::{self, Alignment, Background, Border, Color, Length, Point, Rectangle, Size, Subscription, mouse};
use cosmic::prelude::*;
use cosmic::widget::{self, button, canvas, container, popover, text};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const STRIP_W: f32 = 180.0;
const FADER_H: f32 = 180.0;

/// Ordres reçus du tray (`show`, `quit`), relevés à chaque tic.
static ORDERS: Mutex<Vec<Order>> = Mutex::new(Vec::new());

/// Écoute les ordres adressés à la fenêtre.
pub fn listen_orders(listener: std::os::unix::net::UnixListener) {
    instance::listen(listener, |order| ORDERS.lock().unwrap().push(order));
}

pub fn run(spawn: bool) -> i32 {
    let settings = cosmic::app::Settings::default()
        .size(Size::new(1600.0, 740.0))
        .size_limits(iced::Limits::NONE.min_width(520.0).min_height(420.0))
        // Coins arrondis de COSMIC = fenêtre transparente, sûre seulement sous
        // Wayland (compositeur garanti) : sous X11 sans compositeur, une
        // fenêtre transparente s'affiche noire.
        .transparent(std::env::var_os("WAYLAND_DISPLAY").is_some());
    match cosmic::app::run::<App>(settings, spawn) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("veemeeter: cannot open the window: {e}");
            1
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Popup {
    Add,
    Vee,
    Fx(String),
    Strip(String),
    Send(String, String),
}

#[derive(Clone, Debug)]
pub enum Message {
    Tick,
    Cmd(Command),
    Popup(Option<Popup>),
    AddStrip(Kind),
    ToggleApps,
    StartOnTray(bool),
    Open(&'static str),
    Fader(String, f32),
    FaderDone(String),
    SendLevel(String, String, f32),
    SendDone(String, String),
    Eq(String, usize, f32),
    EqDone(String),
    RenameInput(String, String),
    RenameSubmit(String),
}

pub struct App {
    core: Core,
    shared: Arc<Mutex<Shared>>,
    snap: Option<Snapshot>,
    connected: bool,
    status: String,
    error: Option<String>,
    meters: Meters,
    last: Instant,
    mimi: HashMap<Mood, widget::image::Handle>,
    popup: Option<Popup>,
    /// Valeurs pendant un glissé (l'écho du daemon arrive après).
    dragging: HashMap<String, f32>,
    send_drag: HashMap<(String, String), f32>,
    eq_drag: HashMap<String, [f32; 5]>,
    rename: HashMap<String, String>,
    show_apps: bool,
    settings: settings::Settings,
}

impl App {
    fn send(&self, cmd: Command) {
        self.shared.lock().unwrap().send(&cmd);
    }

    fn mood(&self) -> Mood {
        if self.error.is_some() {
            Mood::Surprised
        } else if !self.connected {
            Mood::Sleepy
        } else if self.meters.loudest() > meter_pos_db(-12.0) {
            Mood::Happy
        } else {
            Mood::Wave
        }
    }

    fn mimi(&self, mood: Mood, size: f32) -> Element<'_, Message> {
        widget::image(self.mimi[&mood].clone()).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
    }

    /// Ouvre un popup, ou le ferme s'il était déjà ouvert.
    fn toggle(&mut self, p: Option<Popup>) {
        self.popup = if p.is_some() && p == self.popup { None } else { p };
    }
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = bool;
    type Message = Message;
    const APP_ID: &'static str = "veemeeter";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, spawn: bool) -> (Self, Task<Message>) {
        let shared = Shared::new();
        link::start_connection(shared.clone(), || {}, spawn);
        let mimi = Mood::ALL
            .into_iter()
            .map(|m| {
                let ([w, h], rgba) = decode_rgba(m.png());
                (m, widget::image::Handle::from_rgba(w as u32, h as u32, rgba))
            })
            .collect();
        let mut app = App {
            core,
            shared,
            snap: None,
            connected: false,
            status: "connecting…".into(),
            error: None,
            meters: Meters::default(),
            last: Instant::now(),
            mimi,
            popup: None,
            dragging: HashMap::new(),
            send_drag: HashMap::new(),
            eq_drag: HashMap::new(),
            rename: HashMap::new(),
            show_apps: true,
            settings: settings::Settings::load(),
        };
        app.set_header_title("VeeMeeter".into());
        let title = app.core.main_window_id().map(|id| app.set_window_title("VeeMeeter".into(), id));
        (app, title.unwrap_or_else(Task::none))
    }

    fn subscription(&self) -> Subscription<Message> {
        // 30 images/s tant qu'un VU-mètre bouge, 10 quand tout est immobile
        // (changements d'état du daemon), 4 en attendant le daemon.
        let period = match (self.connected, self.meters.animating()) {
            (true, true) => 33,
            (true, false) => 100,
            (false, _) => 250,
        };
        iced::time::every(Duration::from_millis(period)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let levels = {
                    let mut s = self.shared.lock().unwrap();
                    if s.error.as_ref().is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(6)) {
                        s.error = None;
                    }
                    self.snap = s.state.clone();
                    self.connected = s.connected;
                    self.status = s.status.clone();
                    self.error = s.error.as_ref().map(|e| e.0.clone());
                    let fresh = s.meters_at.is_some_and(|t| t.elapsed() < Duration::from_millis(300));
                    if fresh { s.meters.clone() } else { HashMap::new() }
                };
                let dt = self.last.elapsed().as_secs_f32().min(0.2);
                self.last = Instant::now();
                self.meters.update(&levels, dt);
                let orders: Vec<Order> = std::mem::take(&mut *ORDERS.lock().unwrap());
                let mut tasks = vec![];
                for order in orders {
                    match order {
                        Order::Show => {
                            if let Some(id) = self.core.main_window_id() {
                                tasks.push(iced::window::minimize(id, false));
                                tasks.push(iced::window::gain_focus(id));
                            }
                        }
                        Order::Quit => tasks.push(iced::exit()),
                    }
                }
                return Task::batch(tasks);
            }
            Message::Cmd(cmd) => {
                if matches!(cmd, Command::Remove { .. }) {
                    self.popup = None;
                }
                self.send(cmd);
            }
            Message::Popup(p) => {
                if matches!(p, Some(Popup::Vee)) {
                    // Le tray a pu changer l'option depuis le dernier affichage.
                    self.settings = settings::Settings::load();
                }
                self.toggle(p);
            }
            Message::AddStrip(kind) => {
                self.popup = None;
                let st = self.snap.as_ref();
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
            Message::ToggleApps => self.show_apps = !self.show_apps,
            Message::StartOnTray(on) => {
                self.settings.start_on_tray = on;
                settings::Settings::set_start_on_tray(on);
            }
            Message::Open(url) => {
                self.popup = None;
                tray::open_url(url);
            }
            Message::Fader(id, db) => {
                let db = (db * 10.0).round() / 10.0;
                self.dragging.insert(id.clone(), db);
                self.send(Command::Volume { id, db });
            }
            Message::FaderDone(id) => {
                self.dragging.remove(&id);
            }
            Message::SendLevel(from, to, db) => {
                let db = (db * 10.0).round() / 10.0;
                self.send_drag.insert((from.clone(), to.clone()), db);
                self.send(Command::Send { from, to, db });
            }
            Message::SendDone(from, to) => {
                self.send_drag.remove(&(from, to));
            }
            Message::Eq(id, band, db) => {
                let current = self.snap.as_ref().and_then(|s| s.strip(&id)).map(|s| s.strip.fx.gains);
                let mut gains = self.eq_drag.get(&id).copied().or(current).unwrap_or_default();
                gains[band] = (db * 2.0).round() / 2.0;
                self.eq_drag.insert(id.clone(), gains);
                self.send(Command::Eq { id, enabled: None, gains: Some(gains) });
            }
            Message::EqDone(id) => {
                self.eq_drag.remove(&id);
            }
            Message::RenameInput(id, v) => {
                self.rename.insert(id, v);
            }
            Message::RenameSubmit(id) => {
                if let Some(name) = self.rename.remove(&id) {
                    let name = name.trim().to_string();
                    let current = self.snap.as_ref().and_then(|s| s.strip(&id)).map(|s| s.strip.name.clone());
                    if !name.is_empty() && Some(&name) != current.as_ref() {
                        self.send(Command::Rename { id, name });
                    }
                }
            }
        }
        Task::none()
    }

    fn header_start(&self) -> Vec<Element<'_, Message>> {
        let add = {
            let b = button::text("+ Add").on_press_maybe(self.connected.then_some(Message::Popup(Some(Popup::Add))));
            let mut p = popover(b).position(popover::Position::Bottom).on_close(Message::Popup(None));
            if self.popup == Some(Popup::Add) {
                let mut col = widget::column::with_capacity(5).spacing(4);
                for kind in [Kind::HardwareIn, Kind::VirtualIn, Kind::Cable, Kind::HardwareOut, Kind::VirtualOut] {
                    col = col.push(button::text(kind.label()).on_press(Message::AddStrip(kind)).width(Length::Fill));
                }
                p = p.popup(menu_box(col.width(Length::Fixed(200.0)).into()));
            }
            p
        };
        let apps = if self.show_apps { button::suggested("Applications") } else { button::standard("Applications") };
        let vee = {
            let b = button::text("Vee").on_press(Message::Popup(Some(Popup::Vee)));
            let mut p = popover(b).position(popover::Position::Bottom).on_close(Message::Popup(None));
            if self.popup == Some(Popup::Vee) {
                let col = widget::column::with_capacity(5)
                    .spacing(6)
                    .push(
                        widget::checkbox(self.settings.start_on_tray)
                            .label("Start on tray")
                            .on_toggle(Message::StartOnTray),
                    )
                    .push(text::caption("Mimi starts in the system tray when you log in; click her to open the mixer"))
                    .push(widget::divider::horizontal::light())
                    .push(button::text("Open GitHub").on_press(Message::Open(tray::REPO_URL)))
                    .push(button::text("Report an issue").on_press(Message::Open(tray::ISSUE_URL)));
                p = p.popup(menu_box(col.width(Length::Fixed(240.0)).into()));
            }
            p
        };
        vec![
            widget::tooltip(
                self.mimi(self.mood(), 28.0),
                text::caption("Mimi, VeeMeeter's sound sprite"),
                widget::tooltip::Position::Bottom,
            )
            .into(),
            add.into(),
            apps.on_press(Message::ToggleApps).into(),
            vee.into(),
        ]
    }

    fn header_end(&self) -> Vec<Element<'_, Message>> {
        let (dot, label) = match (self.connected, &self.snap) {
            (true, Some(s)) => (T::METER_LOW, format!("PipeWire {}", s.pipewire_version)),
            _ => (T::METER_MID, self.status.clone()),
        };
        let mut items: Vec<Element<'_, Message>> = vec![];
        if let Some(e) = &self.error {
            items.push(colored(text::caption(format!("⚠ {e}")).into(), T::MUTE));
        }
        if self.connected {
            items.push(
                widget::tooltip(
                    button::text("Stop engine").on_press(Message::Cmd(Command::Quit)),
                    text::caption("Stops the daemon: every VeeMeeter device disappears until it is started again"),
                    widget::tooltip::Position::Bottom,
                )
                .into(),
            );
        }
        items.push(
            widget::row::with_capacity(2)
                .spacing(6)
                .align_y(Alignment::Center)
                .push(dot_widget(dot, 8.0))
                .push(text::caption(label))
                .into(),
        );
        items
    }

    fn view(&self) -> Element<'_, Message> {
        let Some(st) = &self.snap else {
            return container(
                widget::column::with_capacity(3)
                    .spacing(8)
                    .align_x(Alignment::Center)
                    .push(self.mimi(Mood::Sleepy, 192.0))
                    .push(text::title4("Mimi is waking the audio engine up…"))
                    .push(text::caption(&self.status)),
            )
            .center(Length::Fill)
            .into();
        };
        let mut col = widget::column::with_capacity(3).spacing(10).push(self.mixer(st));
        if self.show_apps {
            col = col.push(widget::divider::horizontal::default()).push(self.apps_panel(st));
        }
        container(col).padding(12).width(Length::Fill).height(Length::Fill).into()
    }
}

impl App {
    fn mixer<'a>(&'a self, st: &'a Snapshot) -> Element<'a, Message> {
        let in_group = |k: Kind, g: usize| match g {
            0 => matches!(k, Kind::HardwareIn | Kind::VirtualIn),
            1 => k == Kind::Cable,
            _ => !k.is_input(),
        };
        let has_cables = st.strips.iter().any(|s| s.strip.kind == Kind::Cable);
        let mut groups = widget::row::with_capacity(5).spacing(16);
        for (g, title) in [(0, "INPUTS"), (1, "CABLES"), (2, "OUTPUTS")] {
            if g == 1 && !has_cables {
                continue;
            }
            if g > 0 {
                groups = groups.push(widget::divider::vertical::default());
            }
            let mut cards = widget::row::with_capacity(8).spacing(10);
            let strips: Vec<&StripState> = st.strips.iter().filter(|s| in_group(s.strip.kind, g)).collect();
            if strips.is_empty() {
                cards = cards.push(text::body("Nothing here yet, use + Add"));
            }
            for s in strips {
                cards = cards.push(self.strip_card(st, s));
            }
            groups =
                groups.push(widget::column::with_capacity(2).spacing(6).push(text::caption_heading(title)).push(cards));
        }
        // Défilable dans les deux sens : beaucoup de strips, ou une petite fenêtre.
        use cosmic::iced::widget::scrollable::{Direction, Scrollbar};
        let bar = || Scrollbar::new().width(8).scroller_width(8);
        widget::scrollable(groups)
            .direction(Direction::Both { vertical: bar(), horizontal: bar() })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn strip_card<'a>(&'a self, st: &'a Snapshot, s: &'a StripState) -> Element<'a, Message> {
        let x = &s.strip;
        let id = x.id.clone();
        let kind_color = match x.kind {
            Kind::Cable => T::CABLE,
            k if k.is_hardware() => T::HARDWARE,
            _ => T::VIRTUAL,
        };
        let tag = if x.kind.is_input() { x.id.to_uppercase() } else { st.bus_label(&x.id) };

        // En-tête : étiquette, FX, menu.
        let fx_active = x.fx.eq || (x.fx.denoise && st.denoise_available);
        let fx_button = pill("FX".into(), fx_active.then_some(T::CABLE), Message::Popup(Some(Popup::Fx(id.clone()))));
        let mut fx = popover(widget::tooltip(
            fx_button,
            text::caption("Equalizer and noise suppression"),
            widget::tooltip::Position::Top,
        ))
        .position(popover::Position::Bottom)
        .on_close(Message::Popup(None));
        if self.popup == Some(Popup::Fx(id.clone())) {
            fx = fx.popup(menu_box(self.fx_menu(st, s)));
        }
        let mut menu = popover(pill("•••".into(), None, Message::Popup(Some(Popup::Strip(id.clone())))))
            .position(popover::Position::Bottom)
            .on_close(Message::Popup(None));
        if self.popup == Some(Popup::Strip(id.clone())) {
            menu = menu.popup(menu_box(self.strip_menu(st, s)));
        }
        let header = widget::row::with_capacity(4)
            .spacing(4)
            .align_y(Alignment::Center)
            .push(badge(tag, kind_color))
            .push(widget::space::horizontal())
            .push(fx)
            .push(menu);

        let kind_name = match x.kind {
            Kind::Cable => "cable",
            k if k.is_hardware() => "hardware",
            _ => "virtual",
        };
        let mut col = widget::column::with_capacity(12)
            .spacing(6)
            .push(header)
            .push(text::heading(&x.name))
            .push(text::caption(format!("{kind_name} · {}", layout_name(&x.channels))));
        col = if x.kind.is_hardware() {
            col.push(self.device_picker(st, s))
        } else {
            col.push(widget::space().height(Length::Fixed(32.0)))
        };
        let state_line: Element<'a, Message> = if !s.ready {
            colored(text::caption("starting…").into(), T::METER_MID)
        } else if x.kind.is_hardware() && !s.online {
            colored(text::caption("device unplugged").into(), T::MUTE)
        } else {
            widget::space().height(Length::Fixed(16.0)).into()
        };
        col = col.push(state_line);

        // VU-mètres + fader.
        let db = self.dragging.get(&x.id).copied().unwrap_or(x.volume_db);
        let channels = x.channels.len().clamp(1, 8);
        let bars: Vec<(f32, f32)> =
            (0..channels).map(|c| self.meters.get(&x.id, c).map(|m| (m.level, m.peak)).unwrap_or((0.0, 0.0))).collect();
        let meter_w = channels as f32 * if channels > 2 { 9.0 } else { 12.0 } + 4.0;
        let meter = canvas(Vu { bars, muted: x.mute }).width(Length::Fixed(meter_w)).height(Length::Fixed(FADER_H));
        let fid = x.id.clone();
        let fader = widget::vertical_slider(MIN_DB..=MAX_DB, db, move |v| Message::Fader(fid.clone(), v))
            .step(0.1f32)
            .on_release(Message::FaderDone(x.id.clone()))
            .height(Length::Fixed(FADER_H));
        col = col.push(widget::row::with_capacity(2).spacing(10).align_y(Alignment::Center).push(meter).push(fader));
        let db_text = if db <= MIN_DB { "−∞ dB".to_string() } else { format!("{db:+.1} dB") };
        col = col.push(container(text::monotext(db_text)).center_x(Length::Fill));

        // Mute + défaut.
        let mut buttons = widget::row::with_capacity(2).spacing(6).push(pill(
            "MUTE".into(),
            x.mute.then_some(T::MUTE),
            Message::Cmd(Command::Mute { id: id.clone(), mute: Some(!x.mute) }),
        ));
        if let Some(sink_role) = x.kind.default_role() {
            let label = match (s.is_default, sink_role) {
                (true, true) => "DEFAULT OUT",
                (true, false) => "DEFAULT MIC",
                (false, true) => "default out",
                (false, false) => "default mic",
            };
            let tip = match (sink_role, x.kind.is_hardware()) {
                (true, false) => "Make this the system default output (apps play here)",
                (true, true) => {
                    "Make this the system default output: apps play through this output's fader and effects"
                }
                (false, false) => "Make this the system default microphone",
                (false, true) => {
                    "Make this the system default microphone: apps hear it after its fader, mute and effects"
                }
            };
            buttons = buttons.push(widget::tooltip(
                pill(label.into(), s.is_default.then_some(T::STAR), Message::Cmd(Command::Primary { id: id.clone() })),
                text::caption(tip),
                widget::tooltip::Position::Top,
            ));
        }
        col = col.push(buttons);

        // Routes A/B (clic droit : niveau d'envoi).
        if x.kind.is_input() {
            let outs: Vec<&StripState> = st.strips.iter().filter(|o| !o.strip.kind.is_input()).collect();
            let mut routes = widget::column::with_capacity(outs.len() / 3 + 1).spacing(4);
            for chunk in outs.chunks(3) {
                let mut r = widget::row::with_capacity(3).spacing(4);
                for out in chunk {
                    r = r.push(self.route_button(st, x, &out.strip));
                }
                routes = routes.push(r);
            }
            col = col.push(routes);
        }

        let border = if s.is_default { Some(T::STAR) } else { None };
        container(col)
            .padding(10)
            .width(Length::Fixed(STRIP_W))
            .style(move |theme: &cosmic::Theme| {
                let c = theme.cosmic();
                container::Style {
                    background: Some(Background::Color(c.primary(false).base.into())),
                    text_color: Some(c.primary(false).on.into()),
                    border: Border {
                        radius: c.corner_radii.radius_m.into(),
                        width: 1.5,
                        color: border.unwrap_or_else(|| c.primary(false).divider.into()),
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    fn route_button<'a>(&'a self, st: &'a Snapshot, from: &'a Strip, to: &'a Strip) -> Element<'a, Message> {
        let on = from.routes.contains(&to.id);
        let c = if to.kind.is_hardware() { T::HARDWARE } else { T::VIRTUAL };
        let label = match from.sends.get(&to.id) {
            Some(db) if *db <= MIN_DB => format!("{} −∞", st.bus_label(&to.id)),
            Some(db) => format!("{} {db:+.0}", st.bus_label(&to.id)),
            None => st.bus_label(&to.id),
        };
        let b = pill(
            label,
            on.then_some(c),
            Message::Cmd(Command::Route { from: from.id.clone(), to: to.id.clone(), on: Some(!on) }),
        );
        let area =
            widget::mouse_area(b).on_right_press(Message::Popup(Some(Popup::Send(from.id.clone(), to.id.clone()))));
        let tip = widget::tooltip(
            area,
            text::caption(format!("Send to {} — right-click for the send level", to.name)),
            widget::tooltip::Position::Top,
        );
        let mut p = popover(tip).position(popover::Position::Bottom).on_close(Message::Popup(None));
        if self.popup == Some(Popup::Send(from.id.clone(), to.id.clone())) {
            p = p.popup(menu_box(self.send_menu(from, to)));
        }
        p.into()
    }

    fn device_picker<'a>(&'a self, st: &'a Snapshot, s: &'a StripState) -> Element<'a, Message> {
        let x = &s.strip;
        let capture = x.kind == Kind::HardwareIn;
        let mut labels = vec!["No device".to_string()];
        let mut names: Vec<Option<String>> = vec![None];
        for d in st.devices.iter().filter(|d| d.capture == capture) {
            labels.push(d.description.clone());
            names.push(Some(d.name.clone()));
        }
        let selected = match &x.device {
            None => Some(0),
            Some(dev) => match names.iter().position(|n| n.as_deref() == Some(dev)) {
                Some(i) => Some(i),
                None => {
                    // Périphérique débranché : il reste affiché, sélectionné.
                    labels.push("(unplugged)".into());
                    names.push(Some(dev.clone()));
                    Some(labels.len() - 1)
                }
            },
        };
        let id = x.id.clone();
        widget::dropdown(labels, selected, move |i| {
            Message::Cmd(Command::SetDevice { id: id.clone(), device: names[i].clone() })
        })
        .width(Length::Fill)
        .into()
    }

    fn send_menu<'a>(&'a self, from: &'a Strip, to: &'a Strip) -> Element<'a, Message> {
        let key = (from.id.clone(), to.id.clone());
        let db = self.send_drag.get(&key).copied().or_else(|| from.sends.get(&to.id).copied()).unwrap_or(0.0);
        let (f, t) = (from.id.clone(), to.id.clone());
        let slider = widget::slider(MIN_DB..=MAX_DB, db, move |v| Message::SendLevel(f.clone(), t.clone(), v))
            .step(0.1f32)
            .on_release(Message::SendDone(from.id.clone(), to.id.clone()))
            .width(Length::Fixed(220.0));
        let value = if db <= MIN_DB { "−∞ dB".into() } else { format!("{db:+.1} dB") };
        widget::column::with_capacity(5)
            .spacing(8)
            .push(text::heading(format!("{} → {}", from.name, to.name)))
            .push(slider)
            .push(text::monotext(value))
            .push(button::text("Back to 0 dB").on_press(Message::Cmd(Command::Send {
                from: from.id.clone(),
                to: to.id.clone(),
                db: 0.0,
            })))
            .push(text::caption("Only this route: the input's fader still applies."))
            .width(Length::Fixed(240.0))
            .into()
    }

    fn fx_menu<'a>(&'a self, st: &'a Snapshot, s: &'a StripState) -> Element<'a, Message> {
        let x = &s.strip;
        let gains = self.eq_drag.get(&x.id).copied().unwrap_or(x.fx.gains);
        let id = x.id.clone();
        let eq_box = widget::checkbox(x.fx.eq)
            .label("Equalizer")
            .on_toggle(move |on| Message::Cmd(Command::Eq { id: id.clone(), enabled: Some(on), gains: None }));
        let mut bands = widget::row::with_capacity(5).spacing(12);
        for (i, (_, _, _, label)) in EQ_BANDS.iter().enumerate() {
            let id = x.id.clone();
            let slider =
                widget::vertical_slider(-EQ_RANGE..=EQ_RANGE, gains[i], move |v| Message::Eq(id.clone(), i, v))
                    .step(0.5f32)
                    .on_release(Message::EqDone(x.id.clone()))
                    .height(Length::Fixed(120.0));
            bands = bands.push(
                widget::column::with_capacity(3)
                    .spacing(4)
                    .align_x(Alignment::Center)
                    .push(slider)
                    .push(text::caption(format!("{:+.0}", gains[i])))
                    .push(text::caption_heading(*label)),
            );
        }
        let flat = button::text("Flat").on_press(Message::Cmd(Command::Eq {
            id: x.id.clone(),
            enabled: None,
            gains: Some([0.0; 5]),
        }));
        let id = x.id.clone();
        let mut denoise = widget::checkbox(x.fx.denoise).label("Noise suppression");
        if st.denoise_available {
            denoise = denoise.on_toggle(move |on| Message::Cmd(Command::Denoise { id: id.clone(), on: Some(on) }));
        }
        let hint = if st.denoise_available {
            "RNNoise: removes background noise (fans, keyboard, hum) from a voice"
        } else {
            "Install the RNNoise LADSPA plugin (librnnoise_ladspa.so) to enable it"
        };
        widget::column::with_capacity(6)
            .spacing(10)
            .push(eq_box)
            .push(bands)
            .push(flat)
            .push(widget::divider::horizontal::light())
            .push(denoise)
            .push(text::caption(hint))
            .width(Length::Fixed(280.0))
            .into()
    }

    fn strip_menu<'a>(&'a self, st: &'a Snapshot, s: &'a StripState) -> Element<'a, Message> {
        let x = &s.strip;
        let value = self.rename.get(&x.id).cloned().unwrap_or_else(|| x.name.clone());
        let (a, b) = (x.id.clone(), x.id.clone());
        let name = widget::text_input("Name", value)
            .on_input(move |v| Message::RenameInput(a.clone(), v))
            .on_submit(move |_| Message::RenameSubmit(b.clone()))
            .on_unfocus(Message::RenameSubmit(x.id.clone()));
        let mut layouts = widget::row::with_capacity(LAYOUTS.len()).spacing(4);
        for (name, layout) in LAYOUTS {
            let sel = layout.len() == x.channels.len() && layout.iter().zip(&x.channels).all(|(a, b)| a == b);
            layouts = layouts.push(pill(
                name.to_string(),
                sel.then_some(T::MIMI_LAVENDER),
                Message::Cmd(Command::SetChannels {
                    id: x.id.clone(),
                    channels: layout.iter().map(|c| c.to_string()).collect(),
                }),
            ));
        }
        let mut col = widget::column::with_capacity(10)
            .spacing(8)
            .push(text::caption("Name"))
            .push(name)
            .push(text::caption("Channels"))
            .push(layouts);
        if let Some(dev) = x.device.as_deref().and_then(|d| st.devices.iter().find(|dv| dv.name == d))
            && !dev.channels.is_empty()
            && dev.channels != x.channels
        {
            col = col.push(
                button::text(format!("Match device ({})", layout_name(&dev.channels)))
                    .on_press(Message::Cmd(Command::SetChannels { id: x.id.clone(), channels: dev.channels.clone() })),
            );
        }
        col.push(widget::divider::horizontal::light())
            .push(
                widget::row::with_capacity(2)
                    .spacing(6)
                    .push(
                        button::text("Move left")
                            .on_press(Message::Cmd(Command::Reorder { id: x.id.clone(), delta: -1 })),
                    )
                    .push(
                        button::text("Move right")
                            .on_press(Message::Cmd(Command::Reorder { id: x.id.clone(), delta: 1 })),
                    ),
            )
            .push(
                button::text("Reset volume (0 dB)")
                    .on_press(Message::Cmd(Command::Volume { id: x.id.clone(), db: 0.0 })),
            )
            .push(widget::divider::horizontal::light())
            .push(button::destructive("Remove").on_press(Message::Cmd(Command::Remove { id: x.id.clone() })))
            .width(Length::Fixed(300.0))
            .into()
    }

    fn apps_panel<'a>(&'a self, st: &'a Snapshot) -> Element<'a, Message> {
        let mut col =
            widget::column::with_capacity(st.apps.len() + 1).spacing(6).push(text::caption_heading("APPLICATIONS"));
        if st.apps.is_empty() {
            return col.push(text::body("No application is playing or recording right now")).into();
        }
        for a in &st.apps {
            // Cibles : strips VeeMeeter (un câble se joue ET s'enregistre), puis périphériques.
            let kind = if a.playback { Kind::VirtualIn } else { Kind::VirtualOut };
            let mut labels = vec![];
            let mut targets = vec![];
            for s in st.strips.iter().filter(|s| s.strip.kind == kind || s.strip.kind == Kind::Cable) {
                labels.push(format!("VeeMeeter · {}", s.strip.name));
                targets.push(s.strip.id.clone());
            }
            for d in st.devices.iter().filter(|d| d.capture != a.playback) {
                labels.push(d.description.clone());
                targets.push(d.name.clone());
            }
            let selected = targets.iter().position(|t| match &a.target_strip {
                Some(strip) => strip == t,
                None => a.target.as_deref() == Some(t.as_str()),
            });
            let app = a.id;
            let picker = widget::dropdown(labels, selected, move |i| {
                Message::Cmd(Command::MoveApp { app, to: targets[i].clone() })
            })
            .width(Length::Fixed(280.0));
            col = col.push(
                widget::row::with_capacity(4)
                    .spacing(14)
                    .align_y(Alignment::Center)
                    .push(widget::tooltip(
                        dot_widget(if a.playback { T::MIMI_LAVENDER } else { T::MIMI_PINK }, 9.0),
                        text::caption(if a.playback { "playing" } else { "recording" }),
                        widget::tooltip::Position::Right,
                    ))
                    .push(container(text::heading(&a.name)).width(Length::Fixed(180.0)))
                    .push(container(text::body(&a.media)).width(Length::Fixed(280.0)))
                    .push(picker),
            );
        }
        widget::scrollable(col).height(Length::Fixed(160.0)).into()
    }
}

/// Bouton à aplat de couleur de marque quand il est « allumé », sinon aux
/// couleurs des boutons du thème système.
fn pill<'a>(label: String, lit: Option<Color>, on_press: Message) -> button::Button<'a, Message> {
    let style = move |hovered: bool| {
        move |_focused: bool, theme: &cosmic::Theme| {
            let c = theme.cosmic();
            let (bg, fg) = match lit {
                Some(color) => (color, T::ON_ACCENT),
                None => (Color::from(c.button.base), Color::from(c.button.on)),
            };
            let bg = if hovered { Color { a: bg.a * 0.85, ..bg } } else { bg };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: Some(fg),
                border_radius: c.corner_radii.radius_s.into(),
                ..button::Style::new()
            }
        }
    };
    button::custom(text::caption_heading(label)).padding([4, 8]).on_press(on_press).class(
        cosmic::theme::Button::Custom {
            active: Box::new(style(false)),
            hovered: Box::new(style(true)),
            pressed: Box::new(style(false)),
            disabled: Box::new(|_theme| button::Style::new()),
        },
    )
}

/// Étiquette de strip (HI1, A1, B1…) sur la couleur de son type.
fn badge<'a>(label: String, color: Color) -> Element<'a, Message> {
    container(text::caption_heading(label))
        .padding([1, 6])
        .style(move |_theme: &cosmic::Theme| container::Style {
            background: Some(Background::Color(color)),
            text_color: Some(T::ON_ACCENT),
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

fn dot_widget<'a>(color: Color, size: f32) -> Element<'a, Message> {
    container(widget::space())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_theme: &cosmic::Theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border { radius: (size / 2.0).into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

/// Contenu d'une couleur donnée (le style de conteneur colore son texte).
fn colored<'a>(content: Element<'a, Message>, color: Color) -> Element<'a, Message> {
    container(content)
        .style(move |_theme: &cosmic::Theme| container::Style { text_color: Some(color), ..Default::default() })
        .into()
}

/// Fond des popups, aux couleurs du thème.
fn menu_box<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content).padding(12).class(cosmic::theme::Container::Dropdown).into()
}

/// VU-mètres d'un strip : segments pleins vert / jaune (≥ −12 dB) / rouge
/// (≥ −3 dB), trait de crête, graduations 0 / −12 / −24 / −48.
struct Vu {
    bars: Vec<(f32, f32)>,
    muted: bool,
}

impl canvas::Program<Message, cosmic::Theme, cosmic::Renderer> for Vu {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &cosmic::Renderer,
        theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<cosmic::Renderer>> {
        let mut f = canvas::Frame::new(renderer, bounds.size());
        let n = self.bars.len().max(1) as f32;
        let gap = 3.0;
        let ticks_w = 4.0;
        let bar_w = ((bounds.width - ticks_w - gap * (n - 1.0)) / n).max(2.0);
        let h = bounds.height;
        let y_of = |v: f32| h - v.clamp(0.0, 1.0) * h;
        let segs = [
            (0.0, meter_pos_db(-12.0), T::METER_LOW),
            (meter_pos_db(-12.0), meter_pos_db(-3.0), T::METER_MID),
            (meter_pos_db(-3.0), 1.0, T::METER_HIGH),
        ];
        let ink: Color = theme.cosmic().background(false).on.into();
        for (i, &(level, peak)) in self.bars.iter().enumerate() {
            let x = i as f32 * (bar_w + gap);
            f.fill(&canvas::Path::rounded_rectangle(Point::new(x, 0.0), Size::new(bar_w, h), 3.0.into()), T::METER_BG);
            for (lo, hi, color) in segs {
                let top = level.min(hi);
                if top > lo {
                    let color = if self.muted { T::dim(color) } else { color };
                    f.fill_rectangle(Point::new(x, y_of(top)), Size::new(bar_w, y_of(lo) - y_of(top)), color);
                }
            }
            if peak > 0.01 {
                f.fill_rectangle(Point::new(x, y_of(peak) - 1.0), Size::new(bar_w, 2.0), ink);
            }
        }
        let tick_x = bounds.width - ticks_w + 1.0;
        for db in [0.0, -12.0, -24.0, -48.0] {
            f.fill_rectangle(Point::new(tick_x, y_of(meter_pos_db(db))), Size::new(3.0, 1.0), Color { a: 0.5, ..ink });
        }
        vec![f.into_geometry()]
    }
}

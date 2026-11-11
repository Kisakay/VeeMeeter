//! Le daemon : possède la connexion PipeWire, les nœuds des strips, les liens
//! et les VU-mètres, et sert l'état aux clients (GUI, `ctl`).
//!
//! Modèle d'exécution : tout tourne sur la boucle principale PipeWire. Les
//! callbacks (registre, métadonnées, paramètres des nœuds, IPC, signaux) ne
//! font qu'empiler des événements et armer le timer « pompe » ; la pompe
//! applique les événements puis **réconcilie** : état voulu (config + graphe)
//! contre état réel. Le hotplug, un redémarrage de WirePlumber ou un lien
//! cassé se réparent donc tout seuls, sans cas particulier.
//!
//! Les objets créés ne persistent pas (`object.linger = false`) : quand le
//! daemon s'arrête, PipeWire les retire tous. Rien ne traîne.

mod fx;
mod graph;
mod meter;
mod pod;
pub mod routing;

use crate::config::Config;
use crate::ipc::{self, ClientTx, Inbound, SendError};
use crate::model::*;
use anyhow::{Context, Result};
use graph::Graph;
use meter::Meter;
use pipewire::{self as pw, properties::properties, spa, types::ObjectType};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

const PREFIX: &str = "veemeeter.";
const METER_PERIOD: Duration = Duration::from_millis(33);
const SAVE_DELAY: Duration = Duration::from_secs(1);
/// Un lien demandé qui n'apparaît pas dans ce délai est recréé.
const LINK_RETRY: Duration = Duration::from_secs(3);
/// Après sa création, un nœud peut voir son volume restauré par le
/// gestionnaire de session : pendant ce délai, notre valeur est réimposée.
const VOLUME_GRACE: Duration = Duration::from_secs(3);

fn node_name(id: &str) -> String {
    format!("{PREFIX}{id}")
}

enum Ev {
    Global {
        id: u32,
        type_: ObjectType,
        props: HashMap<String, String>,
    },
    Remove(u32),
    Meta {
        subject: u32,
        key: String,
        value: Option<String>,
    },
    NodeProps {
        strip: String,
        volumes: Vec<f32>,
        mute: bool,
    },
    /// Propriétés complètes d'un nœud tiers (le registre n'en donne qu'une partie).
    NodeInfo {
        id: u32,
        props: HashMap<String, String>,
    },
    CoreInfo {
        version: String,
    },
    Synced,
    Ipc(Inbound),
    Quit,
}

type Queue = Rc<RefCell<VecDeque<Ev>>>;
type Timer = Rc<pw::loop_::TimerSource<'static>>;

/// Empile un événement et réveille la pompe.
#[derive(Clone)]
struct Waker {
    queue: Queue,
    pump: Rc<RefCell<Option<Timer>>>,
}

impl Waker {
    fn push(&self, ev: Ev) {
        self.queue.borrow_mut().push_back(ev);
        if let Some(t) = self.pump.borrow().as_ref() {
            // Petit délai : les rafales du registre sont traitées d'un bloc.
            t.update_timer(Some(Duration::from_millis(3)), None);
        }
    }
}

#[derive(PartialEq, Clone)]
struct NodeSig {
    kind: Kind,
    name: String,
    channels: Vec<String>,
}

struct Owned {
    proxy: pw::node::Node,
    _listener: pw::node::NodeListener,
    sig: NodeSig,
    /// Dernier volume envoyé (linéaire, mute).
    applied: (Vec<f32>, bool),
    /// Le nœud a renvoyé au moins une fois nos valeurs : les changements
    /// suivants viennent de l'extérieur (pavucontrol, touches volume…).
    synced: bool,
    created: Instant,
}

/// Effets chargés pour un strip (module `filter-chain` en process).
struct FxState {
    _module: fx::Module,
    sig: fx::FxSig,
    /// Nœud d'entrée des effets, lié pour régler les gains en direct.
    proxy: Option<pw::node::Node>,
    applied: Option<[f32; 5]>,
}

struct OwnedLink {
    proxy: pw::link::Link,
    at: Instant,
}

struct ClientState {
    tx: ClientTx,
    subscribed: bool,
    meters: bool,
}

struct Engine {
    ml: &'static pw::main_loop::MainLoopRc,
    core: pw::core::CoreRc,
    registry: pw::registry::RegistryRc,
    waker: Waker,
    save_timer: Timer,
    meter_timer: Timer,
    meter_running: bool,

    cfg: Config,
    cfg_path: PathBuf,
    first_run: bool,
    synced: bool,

    graph: Graph,
    nodes: HashMap<String, Owned>,
    /// Nœuds tiers (périphériques, applications) observés pour leurs propriétés.
    watched: HashMap<u32, (pw::node::Node, pw::node::NodeListener)>,
    meters: HashMap<String, Meter>,
    fx: HashMap<String, FxState>,
    /// Plugin LADSPA RNNoise trouvé au démarrage.
    rnnoise: Option<PathBuf>,
    links: HashMap<(u32, u32), OwnedLink>,
    metadata: Option<(u32, pw::metadata::Metadata, pw::metadata::MetadataListener)>,
    defaults: HashMap<String, String>,
    primary_done: HashSet<String>,
    version: String,

    clients: BTreeMap<u64, ClientState>,
    pongs: Vec<(u64, u64)>,
    last: Option<Snapshot>,
    dirty: bool,
    quitting: bool,
    /// En dernier : les modules d'effets (champ `fx`) sont détruits avant lui.
    context: pw::context::ContextRc,
}

pub fn run(cfg_path: PathBuf) -> Result<i32> {
    let (cfg, first_run) = match Config::load(&cfg_path)? {
        Some(c) => (c, false),
        None => (Config::first_run(), true),
    };
    let listener = ipc::bind()?;

    pw::init();
    let ml: &'static pw::main_loop::MainLoopRc =
        Box::leak(Box::new(pw::main_loop::MainLoopRc::new(None).context("pipewire main loop")?));
    let context = pw::context::ContextRc::new(ml, None).context("pipewire context")?;
    let core = context
        .connect_rc(Some(properties! {
            "application.name" => "VeeMeeter",
            "application.id" => "veemeeter",
            "media.category" => "Manager",
        }))
        .context("connecting to PipeWire (is it running?)")?;
    let registry = core.get_registry_rc().context("pipewire registry")?;
    let lp: &'static pw::loop_::Loop = ml.loop_();

    let waker = Waker { queue: Rc::default(), pump: Rc::default() };
    let exit_code = Rc::new(Cell::new(0));

    // IPC : les threads clients passent par un canal rattaché à la boucle.
    let (tx, rx) = pw::channel::channel::<Inbound>();
    ipc::serve(listener, move |m| tx.send(m).is_ok());
    let w = waker.clone();
    let _ipc = rx.attach(lp, move |m| w.push(Ev::Ipc(m)));

    let w = waker.clone();
    let _reg = registry
        .add_listener_local()
        .global(move |g| {
            let props =
                g.props.map(|d| d.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()).unwrap_or_default();
            w.push(Ev::Global { id: g.id, type_: g.type_.clone(), props });
        })
        .global_remove({
            let w = waker.clone();
            move |id| w.push(Ev::Remove(id))
        })
        .register();

    let w = waker.clone();
    let w2 = waker.clone();
    let code = exit_code.clone();
    let _core_listener = core
        .add_listener_local()
        .info(move |info| w.push(Ev::CoreInfo { version: info.version().to_string() }))
        .done(move |id, _seq| {
            if id == pw::core::PW_ID_CORE {
                w2.push(Ev::Synced);
            }
        })
        .error(move |id, _seq, res, message| {
            // -ENOENT : objet déjà retiré (lien d'un flux disparu), sans gravité.
            if res == -2 {
                log::debug!("pipewire: {message} ({id})");
            } else {
                log::warn!("pipewire error on {id}: {message} ({res})");
            }
            if id == pw::core::PW_ID_CORE && res == -libc_epipe() {
                log::error!("lost the PipeWire connection");
                code.set(1);
                ml.quit();
            }
        })
        .register();
    core.sync(0).context("pipewire sync")?;

    let w = waker.clone();
    let _sigint = lp.add_signal_local(pw::loop_::Signal::INT, move || w.push(Ev::Quit));
    let w = waker.clone();
    let _sigterm = lp.add_signal_local(pw::loop_::Signal::TERM, move || w.push(Ev::Quit));

    // Les timers capturent le moteur ; le moteur détient les timers.
    let engine: Rc<RefCell<Option<Engine>>> = Rc::default();
    let e = engine.clone();
    let save_timer: Timer = Rc::new(lp.add_timer(move |_| {
        if let Some(e) = e.borrow_mut().as_mut() {
            e.save();
        }
    }));
    let e = engine.clone();
    let meter_timer: Timer = Rc::new(lp.add_timer(move |_| {
        if let Some(e) = e.borrow_mut().as_mut() {
            e.send_meters();
        }
    }));
    let e = engine.clone();
    let queue = waker.queue.clone();
    let pump: Timer = Rc::new(lp.add_timer(move |_| {
        let events: Vec<Ev> = queue.borrow_mut().drain(..).collect();
        if let Some(e) = e.borrow_mut().as_mut() {
            e.pump(events);
        }
    }));
    *waker.pump.borrow_mut() = Some(pump);

    *engine.borrow_mut() = Some(Engine {
        ml,
        core,
        registry,
        waker: waker.clone(),
        save_timer,
        meter_timer,
        meter_running: false,
        cfg,
        cfg_path,
        first_run,
        synced: false,
        graph: Graph::default(),
        nodes: HashMap::new(),
        watched: HashMap::new(),
        meters: HashMap::new(),
        fx: HashMap::new(),
        rnnoise: fx::find_rnnoise(),
        links: HashMap::new(),
        metadata: None,
        defaults: HashMap::new(),
        primary_done: HashSet::new(),
        version: String::new(),
        clients: BTreeMap::new(),
        pongs: vec![],
        last: None,
        dirty: true,
        quitting: false,
        context,
    });
    log::info!("VeeMeeter daemon listening on {}", ipc::socket_path().display());
    ml.run();

    // Arrêt : la config est sauvée, les proxys sont rendus (PipeWire retire
    // nœuds et liens non persistants avec la connexion).
    if let Some(mut e) = engine.borrow_mut().take() {
        e.save();
    }
    let _ = std::fs::remove_file(ipc::socket_path());
    Ok(exit_code.get())
}

fn libc_epipe() -> i32 {
    32 // EPIPE (Linux)
}

impl Engine {
    fn pump(&mut self, events: Vec<Ev>) {
        for ev in events {
            self.handle(ev);
        }
        if self.quitting {
            self.save();
            self.ml.quit();
            return;
        }
        if self.dirty {
            self.dirty = false;
            self.reconcile();
        }
        self.publish();
    }

    fn handle(&mut self, ev: Ev) {
        match ev {
            Ev::Global { id, type_, props } => {
                let get = |k: &str| props.get(k).cloned();
                match type_ {
                    ObjectType::Node => {
                        self.graph.add_node(id, &get);
                        self.watch_node(id, &props);
                        self.bind_fx(id, &props);
                    }
                    ObjectType::Port => self.graph.add_port(id, &get),
                    ObjectType::Link => self.graph.add_link(id, &get),
                    ObjectType::Metadata if get("metadata.name").as_deref() == Some("default") => {
                        self.bind_metadata(id, &props)
                    }
                    _ => return,
                }
                self.dirty = true;
            }
            Ev::Remove(id) => {
                self.watched.remove(&id);
                if self.graph.remove(id) {
                    self.dirty = true;
                }
                if self.metadata.as_ref().is_some_and(|m| m.0 == id) {
                    self.metadata = None;
                }
            }
            Ev::Meta { subject, key, value } => {
                if subject == 0 && key.starts_with("default.") {
                    match value.and_then(|v| json_name(&v)) {
                        Some(name) => self.defaults.insert(key, name),
                        None => self.defaults.remove(&key),
                    };
                    self.dirty = true;
                }
            }
            Ev::NodeProps { strip, volumes, mute } => self.on_node_props(&strip, volumes, mute),
            Ev::NodeInfo { id, props } => {
                if self.graph.nodes.contains_key(&id) && props.contains_key("node.name") {
                    self.graph.add_node(id, &|k: &str| props.get(k).cloned());
                    self.dirty = true;
                }
            }
            Ev::CoreInfo { version } => self.version = version,
            Ev::Synced => {
                if !self.synced {
                    self.synced = true;
                    if self.first_run {
                        self.fill_first_run_devices();
                    }
                    self.dirty = true;
                }
            }
            Ev::Ipc(Inbound::Join { client, tx }) => {
                self.clients.insert(client, ClientState { tx, subscribed: false, meters: false });
            }
            Ev::Ipc(Inbound::Leave { client }) => {
                self.clients.remove(&client);
                self.dirty = true;
            }
            Ev::Ipc(Inbound::Cmd { client, cmd }) => {
                if let Err(text) = self.command(client, cmd) {
                    self.send_to(client, &Message::Error { text });
                }
            }
            Ev::Quit => self.quitting = true,
        }
    }

    fn command(&mut self, client: u64, cmd: Command) -> Result<(), String> {
        match &cmd {
            Command::Subscribe { meters } => {
                if let Some(c) = self.clients.get_mut(&client) {
                    c.subscribed = true;
                    c.meters = *meters;
                }
                let snap = self.snapshot();
                self.send_to(client, &Message::State { state: snap });
                self.dirty = true; // crée/retire les VU-mètres
                return Ok(());
            }
            Command::Ping { tag } => {
                self.pongs.push((client, *tag));
                return Ok(());
            }
            Command::Quit => {
                self.quitting = true;
                return Ok(());
            }
            Command::MoveApp { app, to } => return self.move_app(*app, to),
            _ => {}
        }
        if !self.cfg.apply(&cmd)? {
            return Ok(());
        }
        self.save_timer.update_timer(Some(SAVE_DELAY), None);
        match cmd {
            Command::Volume { id, .. } | Command::Mute { id, .. } => self.apply_volume(&id),
            Command::Primary { id } => {
                self.primary_done.remove(&id);
                self.dirty = true;
            }
            _ => self.dirty = true,
        }
        Ok(())
    }

    // ---------------------------------------------------------------- nœuds

    fn create_node(&self, s: &Strip) -> Result<(pw::node::Node, pw::node::NodeListener), pw::Error> {
        let class = match s.kind {
            // Nœuds internes : invisibles dans pavucontrol et les menus du bureau.
            Kind::HardwareIn | Kind::HardwareOut => "Audio/Sink/Internal",
            Kind::VirtualIn => "Audio/Sink",
            Kind::VirtualOut => "Audio/Source/Virtual",
        };
        let description =
            if s.kind.is_hardware() { format!("VeeMeeter {} ({})", s.name, s.id) } else { s.name.clone() };
        let node = self.core.create_object::<pw::node::Node>(
            "adapter",
            &properties! {
                "factory.name" => "support.null-audio-sink",
                "node.name" => node_name(&s.id),
                "node.description" => description,
                "node.nick" => s.name.as_str(),
                "media.class" => class,
                "audio.channels" => s.channels.len().to_string(),
                "audio.position" => s.channels.join(","),
                "monitor.channel-volumes" => "true",
                "monitor.passthrough" => "true",
                "node.virtual" => "true",
                "object.linger" => "false",
                "device.icon-name" => if s.kind.is_input() { "audio-input-microphone" } else { "audio-speakers" },
                "veemeeter.strip" => s.id.as_str(),
            },
        )?;
        let w = self.waker.clone();
        let strip = s.id.clone();
        let listener = node
            .add_listener_local()
            .param(move |_seq, id, _index, _next, param| {
                if id != spa::param::ParamType::Props {
                    return;
                }
                if let Some((volumes, mute)) = param.and_then(pod::parse_props) {
                    w.push(Ev::NodeProps { strip: strip.clone(), volumes, mute });
                }
            })
            .register();
        node.subscribe_params(&[spa::param::ParamType::Props]);
        Ok((node, listener))
    }

    fn desired_volume(s: &Strip) -> (Vec<f32>, bool) {
        (vec![db_to_linear(s.volume_db); s.channels.len()], s.mute)
    }

    fn apply_volume(&mut self, id: &str) {
        let Some(s) = self.cfg.get(id) else { return };
        let want = Self::desired_volume(s);
        let Some(o) = self.nodes.get_mut(id) else { return };
        let bytes = pod::props(&want.0, want.1);
        o.proxy.set_param(spa::param::ParamType::Props, 0, spa::pod::Pod::from_bytes(&bytes).expect("valid pod"));
        o.applied = want;
        o.synced = false;
    }

    fn on_node_props(&mut self, strip: &str, volumes: Vec<f32>, mute: bool) {
        let Some(o) = self.nodes.get_mut(strip) else { return };
        let same = |a: &[f32], b: &[f32]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3);
        if same(&volumes, &o.applied.0) && mute == o.applied.1 {
            o.synced = true;
            return;
        }
        if !o.synced {
            return; // pas encore nos valeurs : état initial du nœud
        }
        if o.created.elapsed() < VOLUME_GRACE {
            // Restauration d'état du gestionnaire de session : la config fait foi.
            self.apply_volume(strip);
            return;
        }
        // Changement extérieur (bureau, pavucontrol, wpctl) : le fader suit.
        let peak = volumes.iter().copied().fold(0.0, f32::max);
        o.applied = (volumes, mute);
        if let Some(s) = self.cfg.strips.iter_mut().find(|s| s.id == strip) {
            s.volume_db = linear_to_db(peak);
            s.mute = mute;
            self.save_timer.update_timer(Some(SAVE_DELAY), None);
        }
    }

    // ------------------------------------------------------------ métadonnées

    fn bind<T: pw::proxy::ProxyT>(
        &self,
        id: u32,
        type_: ObjectType,
        props: &HashMap<String, String>,
    ) -> Result<T, pw::Error> {
        let mut dict = pw::properties::PropertiesBox::new();
        for (k, v) in props {
            dict.insert(k.as_str(), v.as_str());
        }
        let global = pw::registry::GlobalObject {
            id,
            permissions: pw::permissions::PermissionFlags::all(),
            type_,
            version: 3,
            props: Some(dict),
        };
        self.registry.bind(&global)
    }

    /// Lie un nœud audio tiers pour recevoir toutes ses propriétés
    /// (`stream.monitor`, `application.name`, titre du média…).
    fn watch_node(&mut self, id: u32, props: &HashMap<String, String>) {
        let class = props.get("media.class").map(String::as_str).unwrap_or("");
        let ours = props.get("node.name").is_some_and(|n| n.starts_with(PREFIX));
        if ours || !(class.starts_with("Stream/") || class.starts_with("Audio/")) || self.watched.contains_key(&id) {
            return;
        }
        let node: pw::node::Node = match self.bind(id, ObjectType::Node, props) {
            Ok(n) => n,
            Err(e) => {
                log::debug!("cannot bind node {id}: {e}");
                return;
            }
        };
        let w = self.waker.clone();
        let listener = node
            .add_listener_local()
            .info(move |info| {
                // Les autres `info` (état, ports) arrivent sans propriétés.
                if !info.change_mask().contains(pw::node::NodeChangeMask::PROPS) {
                    return;
                }
                if let Some(d) = info.props() {
                    let props = d.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
                    w.push(Ev::NodeInfo { id, props });
                }
            })
            .register();
        self.watched.insert(id, (node, listener));
    }

    /// Lie le nœud d'entrée des effets d'un strip, pour régler les gains.
    fn bind_fx(&mut self, id: u32, props: &HashMap<String, String>) {
        let Some(strip) = props
            .get("node.name")
            .and_then(|n| n.strip_prefix("veemeeter.fx."))
            .and_then(|n| n.strip_suffix(".in"))
            .map(str::to_string)
        else {
            return;
        };
        if !self.fx.get(&strip).is_some_and(|f| f.proxy.is_none()) {
            return;
        }
        match self.bind::<pw::node::Node>(id, ObjectType::Node, props) {
            Ok(node) => {
                if let Some(f) = self.fx.get_mut(&strip) {
                    f.proxy = Some(node);
                    f.applied = None; // réappliqués à la prochaine réconciliation
                }
                self.dirty = true;
            }
            Err(e) => log::debug!("cannot bind effects of {strip}: {e}"),
        }
    }

    fn bind_metadata(&mut self, id: u32, props: &HashMap<String, String>) {
        let meta: pw::metadata::Metadata = match self.bind(id, ObjectType::Metadata, props) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("cannot bind default metadata: {e}");
                return;
            }
        };
        let w = self.waker.clone();
        let listener = meta
            .add_listener_local()
            .property(move |subject, key, _type, value| {
                if let Some(key) = key {
                    w.push(Ev::Meta { subject, key: key.to_string(), value: value.map(str::to_string) });
                }
                0
            })
            .register();
        self.metadata = Some((id, meta, listener));
    }

    fn set_default(&mut self, s: &Strip) {
        let Some((_, meta, _)) = &self.metadata else { return };
        let key =
            if s.kind == Kind::VirtualIn { "default.configured.audio.sink" } else { "default.configured.audio.source" };
        let value = serde_json::json!({ "name": node_name(&s.id) }).to_string();
        meta.set_property(0, key, Some("Spa:String:JSON"), Some(&value));
        log::info!("default {} → {}", key.rsplit('.').next().unwrap_or(key), s.name);
    }

    fn move_app(&mut self, app: u32, to: &str) -> Result<(), String> {
        let node = self.graph.nodes.get(&app).ok_or("unknown application stream")?;
        if !node.class.starts_with("Stream/") || node.name.starts_with(PREFIX) {
            return Err("not an application stream".into());
        }
        let target_name = if self.cfg.get(to).is_some() { node_name(to) } else { to.to_string() };
        let target = self.graph.node_by_name(&target_name).ok_or("target is not available")?;
        let serial = target.serial.ok_or("target has no serial")?;
        let (_, meta, _) = self.metadata.as_ref().ok_or("PipeWire default metadata is not available")?;
        // Même procédé que pipewire-pulse (pavucontrol) : WirePlumber relie le flux.
        meta.set_property(app, "target.node", None, None);
        meta.set_property(app, "target.object", Some("Spa:Id"), Some(&serial.to_string()));
        Ok(())
    }

    /// Premier lancement : branche le micro et la sortie par défaut du système.
    fn fill_first_run_devices(&mut self) {
        let pick = |capture: bool, this: &Engine| -> Option<String> {
            let key = if capture { "default.audio.source" } else { "default.audio.sink" };
            let class = if capture { "Audio/Source" } else { "Audio/Sink" };
            this.defaults.get(key).filter(|n| !n.starts_with(PREFIX)).cloned().or_else(|| {
                this.graph
                    .nodes
                    .values()
                    .find(|n| n.class == class && !n.name.starts_with(PREFIX))
                    .map(|n| n.name.clone())
            })
        };
        let (src, sink) = (pick(true, self), pick(false, self));
        for s in &mut self.cfg.strips {
            if s.device.is_none() {
                s.device = match s.kind {
                    Kind::HardwareIn => src.clone(),
                    Kind::HardwareOut => sink.clone(),
                    _ => None,
                };
            }
        }
        self.first_run = false;
        self.save();
    }

    // ---------------------------------------------------------- réconciliation

    fn reconcile(&mut self) {
        // 1. Nœuds des strips.
        let wanted: HashMap<String, NodeSig> = self
            .cfg
            .strips
            .iter()
            .map(|s| (s.id.clone(), NodeSig { kind: s.kind, name: s.name.clone(), channels: s.channels.clone() }))
            .collect();
        let stale: Vec<String> =
            self.nodes.iter().filter(|(id, o)| wanted.get(*id) != Some(&o.sig)).map(|(id, _)| id.clone()).collect();
        for id in stale {
            if let Some(o) = self.nodes.remove(&id) {
                let _ = self.core.destroy_object(o.proxy);
            }
            self.meters.remove(&id);
        }
        let strips = self.cfg.strips.clone();
        for s in &strips {
            if self.nodes.contains_key(&s.id) {
                continue;
            }
            match self.create_node(s) {
                Ok((proxy, listener)) => {
                    let sig = wanted[&s.id].clone();
                    self.nodes.insert(
                        s.id.clone(),
                        Owned {
                            proxy,
                            _listener: listener,
                            sig,
                            applied: (vec![], false),
                            synced: false,
                            created: Instant::now(),
                        },
                    );
                    self.apply_volume(&s.id);
                }
                Err(e) => log::warn!("cannot create node for {}: {e}", s.id),
            }
        }

        // 1 bis. Effets : module (re)chargé si leur nature change, gains en direct.
        self.fx.retain(|id, f| {
            strips.iter().find(|s| &s.id == id).and_then(|s| fx::wanted(s, self.rnnoise.as_deref())).as_ref()
                == Some(&f.sig)
        });
        for s in &strips {
            let Some(sig) = fx::wanted(s, self.rnnoise.as_deref()) else { continue };
            if !self.fx.contains_key(&s.id) {
                let args = fx::module_args(s, &sig, self.rnnoise.as_deref());
                match fx::Module::load(&self.context, &args) {
                    Ok(module) => {
                        log::info!("effects loaded on {} (eq: {}, denoise: {})", s.id, sig.eq, sig.denoise);
                        self.fx.insert(s.id.clone(), FxState { _module: module, sig, proxy: None, applied: None });
                        // Le nœud a peut-être déjà été annoncé : on le lie maintenant.
                        if let Some(n) = self.graph.node_by_name(&fx::in_name(&s.id)).map(|n| n.id) {
                            self.bind_fx(n, &HashMap::from([("node.name".into(), fx::in_name(&s.id))]));
                        }
                    }
                    Err(e) => log::warn!("effects on {}: {e}", s.id),
                }
            }
            if let Some(f) = self.fx.get_mut(&s.id) {
                if let (Some(proxy), true) = (&f.proxy, f.sig.eq && f.applied != Some(s.fx.gains)) {
                    let bytes = fx::gains_pod(&s.fx);
                    proxy.set_param(
                        spa::param::ParamType::Props,
                        0,
                        spa::pod::Pod::from_bytes(&bytes).expect("valid pod"),
                    );
                    f.applied = Some(s.fx.gains);
                }
            }
        }

        // 2. Périphérique par défaut (une fois par démarrage / par demande).
        if self.synced && self.metadata.is_some() {
            for s in strips.iter().filter(|s| s.primary) {
                if self.graph.node_by_name(&node_name(&s.id)).is_some() && self.primary_done.insert(s.id.clone()) {
                    self.set_default(s);
                }
            }
        }

        // 3. VU-mètres, seulement si un client les affiche.
        let want_meters = self.clients.values().any(|c| c.meters);
        self.meters.retain(|id, m| want_meters && wanted.get(id).is_some_and(|w| w.channels == m.channels));
        if want_meters {
            for s in &strips {
                if !self.meters.contains_key(&s.id) && self.graph.node_by_name(&node_name(&s.id)).is_some() {
                    match Meter::new(&self.core, &s.id, &s.channels) {
                        Ok(m) => {
                            self.meters.insert(s.id.clone(), m);
                        }
                        Err(e) => log::warn!("meter for {}: {e}", s.id),
                    }
                }
            }
        }
        if want_meters != self.meter_running {
            self.meter_running = want_meters;
            self.meter_timer.update_timer(want_meters.then_some(METER_PERIOD), want_meters.then_some(METER_PERIOD));
        }

        // 4. Liens.
        let desired = self.desired_links();
        let now = Instant::now();
        let mut drop_keys = vec![];
        for (key, l) in &self.links {
            let gone = !self.graph.ports.contains_key(&key.0) || !self.graph.ports.contains_key(&key.1);
            let failed = !self.graph.has_link(key.0, key.1) && now.duration_since(l.at) > LINK_RETRY;
            if !desired.contains_key(key) || gone || failed {
                drop_keys.push(*key);
            }
        }
        for key in drop_keys {
            if let Some(l) = self.links.remove(&key) {
                if desired.contains_key(&key) && !self.graph.has_link(key.0, key.1) {
                    log::debug!("link {key:?} never appeared, retrying");
                }
                let _ = self.core.destroy_object(l.proxy);
            }
        }
        for (key, passive) in desired {
            if self.links.contains_key(&key) || self.graph.has_link(key.0, key.1) {
                continue;
            }
            let (Some(op), Some(ip)) = (self.graph.ports.get(&key.0), self.graph.ports.get(&key.1)) else { continue };
            match self.core.create_object::<pw::link::Link>(
                "link-factory",
                &properties! {
                    "link.output.node" => op.node.to_string(),
                    "link.output.port" => key.0.to_string(),
                    "link.input.node" => ip.node.to_string(),
                    "link.input.port" => key.1.to_string(),
                    "link.passive" => if passive { "true" } else { "false" },
                    "object.linger" => "false",
                },
            ) {
                Ok(proxy) => {
                    self.links.insert(key, OwnedLink { proxy, at: now });
                }
                Err(e) => log::warn!("cannot link {key:?}: {e}"),
            }
        }
    }

    /// Liens voulus `(port sortie, port entrée) → passif`.
    fn desired_links(&self) -> HashMap<(u32, u32), bool> {
        let g = &self.graph;
        let mut out = HashMap::new();
        let ports = |node: u32, output: bool| -> Vec<routing::P<'_>> {
            g.ports_of(node, output)
                .into_iter()
                .filter(|p| !(output && p.monitor))
                .map(|p| routing::P { id: p.id, channel: &p.channel })
                .collect()
        };
        let our_outs = |node: u32| -> Vec<routing::P<'_>> {
            g.ports_of(node, true).into_iter().map(|p| routing::P { id: p.id, channel: &p.channel }).collect()
        };
        let mut add = |pairs: Vec<(u32, u32)>, passive: bool| {
            for k in pairs {
                out.insert(k, passive);
            }
        };
        // Nœuds d'effets (entrée, sortie) d'un strip, s'ils sont prêts.
        let fx_nodes = |id: &str| -> Option<(u32, u32)> {
            if !self.fx.contains_key(id) {
                return None;
            }
            Some((g.node_by_name(&fx::in_name(id))?.id, g.node_by_name(&fx::out_name(id))?.id))
        };
        // Où le son entre dans un strip / d'où il en sort, effets compris.
        let ends = |s: &Strip, node: u32| -> (u32, u32) {
            match (s.kind, fx_nodes(&s.id)) {
                // Sortie B : les effets passent AVANT le micro virtuel.
                (Kind::VirtualOut, Some((fi, _))) => (fi, node),
                (_, Some((_, fo))) => (node, fo),
                (_, None) => (node, node),
            }
        };
        for s in &self.cfg.strips {
            let Some(node) = g.node_by_name(&node_name(&s.id)) else { continue };
            let (sink, source) = ends(s, node.id);
            if let Some((fi, fo)) = fx_nodes(&s.id) {
                if s.kind == Kind::VirtualOut {
                    add(routing::match_ports(&our_outs(fo), &ports(node.id, false)), false);
                } else {
                    add(routing::match_ports(&our_outs(node.id), &ports(fi, false)), false);
                }
            }
            let device = s.device.as_deref().and_then(|d| g.node_by_name(d)).filter(|d| !d.name.starts_with(PREFIX));
            match (s.kind, device) {
                (Kind::HardwareIn, Some(d)) => {
                    add(routing::match_ports(&ports(d.id, true), &ports(sink, false)), false)
                }
                (Kind::HardwareOut, Some(d)) => {
                    add(routing::match_ports(&our_outs(source), &ports(d.id, false)), false)
                }
                _ => {}
            }
            for r in &s.routes {
                let target = self.cfg.get(r).and_then(|t| Some((t, g.node_by_name(&node_name(r))?)));
                if let Some((t, tn)) = target {
                    let (t_sink, _) = ends(t, tn.id);
                    add(routing::match_ports(&our_outs(source), &ports(t_sink, false)), false);
                }
            }
            // Mesure après les effets (pour une sortie B, le nœud lui-même).
            let metered = if s.kind == Kind::VirtualOut { node.id } else { source };
            if let Some(m) = self.meters.get(&s.id).and_then(|m| g.node_by_name(&m.node_name)) {
                let pairs: Vec<(u32, u32)> =
                    our_outs(metered).iter().zip(ports(m.id, false)).map(|(o, i)| (o.id, i.id)).collect();
                add(pairs, true);
            }
        }
        out
    }

    // ------------------------------------------------------------- clients

    fn snapshot(&self) -> Snapshot {
        let g = &self.graph;
        let default_sink = self.defaults.get("default.audio.sink").cloned();
        let default_source = self.defaults.get("default.audio.source").cloned();
        let strips = self
            .cfg
            .strips
            .iter()
            .map(|s| {
                let name = node_name(&s.id);
                StripState {
                    ready: g.node_by_name(&name).is_some(),
                    online: !s.kind.is_hardware() || s.device.as_deref().is_some_and(|d| g.node_by_name(d).is_some()),
                    is_default: default_sink.as_deref() == Some(&name) || default_source.as_deref() == Some(&name),
                    strip: s.clone(),
                }
            })
            .collect();
        let mut devices: Vec<Device> = g
            .nodes
            .values()
            .filter(|n| !n.name.starts_with(PREFIX) && (n.class == "Audio/Sink" || n.class == "Audio/Source"))
            .map(|n| Device {
                name: n.name.clone(),
                description: n.description.clone(),
                capture: n.class == "Audio/Source",
                channels: n.channels.clone(),
            })
            .collect();
        devices.sort_by(|a, b| a.description.cmp(&b.description));
        let strip_of =
            |name: &str| name.strip_prefix(PREFIX).filter(|id| self.cfg.get(id).is_some()).map(str::to_string);
        let mut apps: Vec<App> = g
            .nodes
            .values()
            .filter(|n| {
                (n.class == "Stream/Output/Audio" || n.class == "Stream/Input/Audio")
                    && !n.name.starts_with(PREFIX)
                    && !n.monitor
            })
            .map(|n| {
                let playback = n.class == "Stream/Output/Audio";
                let target = g
                    .links
                    .values()
                    .filter_map(|l| match playback {
                        true if l.out_node == n.id => Some(l.in_node),
                        false if l.in_node == n.id => Some(l.out_node),
                        _ => None,
                    })
                    .filter_map(|id| g.nodes.get(&id))
                    // Un flux est aussi relié aux VU-mètres : seul un périphérique compte.
                    .find(|t| t.class.starts_with("Audio/"))
                    .map(|t| t.name.clone());
                App {
                    id: n.id,
                    name: n.app_name.clone().unwrap_or_else(|| n.description.clone()),
                    media: n.media_name.clone().unwrap_or_default(),
                    icon: n.icon.clone(),
                    playback,
                    target_strip: target.as_deref().and_then(strip_of),
                    target,
                }
            })
            .collect();
        apps.sort_by(|a, b| (!a.playback, &a.name, a.id).cmp(&(!b.playback, &b.name, b.id)));
        Snapshot {
            strips,
            devices,
            apps,
            default_sink,
            default_source,
            pipewire_version: self.version.clone(),
            denoise_available: self.rnnoise.is_some(),
        }
    }

    fn publish(&mut self) {
        if self.clients.values().any(|c| c.subscribed) {
            let snap = self.snapshot();
            if self.last.as_ref() != Some(&snap) {
                let line = ipc::encode(&Message::State { state: snap.clone() });
                self.broadcast(&line, |c| c.subscribed);
                self.last = Some(snap);
            }
        } else {
            self.last = None;
        }
        for (client, tag) in std::mem::take(&mut self.pongs) {
            self.send_to(client, &Message::Pong { tag });
        }
    }

    fn broadcast(&mut self, line: &ipc::Line, filter: impl Fn(&ClientState) -> bool) {
        let mut gone = vec![];
        for (id, c) in &self.clients {
            if filter(c) {
                if let Err(SendError::Gone) = c.tx.send(line.clone()) {
                    gone.push(*id);
                }
            }
        }
        for id in gone {
            self.clients.remove(&id);
            self.dirty = true;
        }
    }

    fn send_to(&mut self, client: u64, msg: &Message) {
        if let Some(c) = self.clients.get(&client) {
            let _ = c.tx.send(ipc::encode(msg));
        }
    }

    fn send_meters(&mut self) {
        let levels: Vec<(String, Vec<f32>)> =
            self.cfg.strips.iter().filter_map(|s| self.meters.get(&s.id).map(|m| (s.id.clone(), m.take()))).collect();
        let line = ipc::encode(&Message::Meters { levels });
        self.broadcast(&line, |c| c.meters);
        if self.dirty {
            self.waker.push(Ev::Synced); // réveille la pompe pour réconcilier
        }
    }

    fn save(&mut self) {
        match self.cfg.save(&self.cfg_path) {
            Ok(()) => log::debug!("config saved to {}", self.cfg_path.display()),
            Err(e) => log::error!("cannot save config: {e:#}"),
        }
    }
}

/// `{"name":"alsa_output…"}` → `alsa_output…`
fn json_name(v: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(v).ok()?.get("name")?.as_str().map(str::to_string)
}

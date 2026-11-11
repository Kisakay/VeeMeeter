//! Socket de contrôle du daemon : `$XDG_RUNTIME_DIR/veemeeter.sock`,
//! une ligne JSON par message dans chaque sens (`model::Command` /
//! `model::Message`).

use crate::model::{Command, Message};
use anyhow::{Context, Result, bail};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::time::Duration;

/// Taille maximale d'une ligne reçue (une commande tient en quelques centaines d'octets).
const MAX_LINE: usize = 64 * 1024;
/// File de sortie par client : au-delà, les trames de niveaux sont abandonnées.
const CLIENT_QUEUE: usize = 256;

pub fn socket_path() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) => PathBuf::from(dir).join("veemeeter.sock"),
        None => PathBuf::from(format!("/tmp/veemeeter-{}.sock", rustix_uid())),
    }
}

fn rustix_uid() -> u32 {
    // Sans dépendance : l'uid se lit dans /proc/self/status.
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("Uid:")).and_then(|l| l.split_whitespace().nth(1)?.parse().ok()))
        .unwrap_or(0)
}

/// Ligne sérialisée partagée entre tous les clients (sérialisée une seule fois).
pub type Line = Arc<str>;

pub fn encode(msg: &Message) -> Line {
    let mut s = serde_json::to_string(msg).expect("message serializes");
    s.push('\n');
    s.into()
}

/// Ce que le serveur transmet au moteur.
pub enum Inbound {
    Join { client: u64, tx: ClientTx },
    Cmd { client: u64, cmd: Command },
    Leave { client: u64 },
}

#[derive(Clone)]
pub struct ClientTx(SyncSender<Line>);

pub enum SendError {
    Full,
    Gone,
}

impl ClientTx {
    pub fn send(&self, line: Line) -> Result<(), SendError> {
        self.0.try_send(line).map_err(|e| match e {
            TrySendError::Full(_) => SendError::Full,
            TrySendError::Disconnected(_) => SendError::Gone,
        })
    }
}

/// Ouvre le socket. Échoue si un daemon répond déjà ; un socket orphelin est
/// supprimé.
pub fn bind() -> Result<UnixListener> {
    let path = socket_path();
    if UnixStream::connect(&path).is_ok() {
        bail!("a VeeMeeter daemon is already running ({})", path.display());
    }
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).with_context(|| format!("binding {}", path.display()))?;
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    Ok(listener)
}

/// Accepte les clients sur un thread dédié et relaie leurs commandes.
pub fn serve(listener: UnixListener, inbound: impl Fn(Inbound) -> bool + Send + Sync + 'static) {
    let inbound = Arc::new(inbound);
    let next = Arc::new(AtomicU64::new(1));
    std::thread::Builder::new()
        .name("ipc-accept".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let client = next.fetch_add(1, Ordering::Relaxed);
                if let Err(e) = spawn_client(client, stream, inbound.clone()) {
                    log::warn!("client {client}: {e:#}");
                }
            }
        })
        .expect("spawn ipc thread");
}

fn spawn_client(
    client: u64,
    stream: UnixStream,
    inbound: Arc<impl Fn(Inbound) -> bool + Send + Sync + 'static>,
) -> Result<()> {
    let (tx, rx) = sync_channel::<Line>(CLIENT_QUEUE);
    let mut writer = stream.try_clone()?;
    writer.set_write_timeout(Some(Duration::from_secs(5)))?;
    std::thread::Builder::new().name(format!("ipc-w{client}")).spawn(move || {
        for line in rx {
            if writer.write_all(line.as_bytes()).is_err() {
                break;
            }
        }
        let _ = writer.shutdown(std::net::Shutdown::Both);
    })?;
    if !inbound(Inbound::Join { client, tx: ClientTx(tx.clone()) }) {
        bail!("engine stopped");
    }
    std::thread::Builder::new().name(format!("ipc-r{client}")).spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        loop {
            line.clear();
            match (&mut reader).take(MAX_LINE as u64).read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) if !line.ends_with('\n') => {
                    let _ = tx.try_send(encode(&Message::Error { text: "line too long".into() }));
                    break;
                }
                Ok(_) => {}
            }
            let text = line.trim();
            if text.is_empty() {
                continue;
            }
            match serde_json::from_str::<Command>(text) {
                Ok(cmd) => {
                    if !inbound(Inbound::Cmd { client, cmd }) {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.try_send(encode(&Message::Error { text: format!("bad command: {e}") }));
                }
            }
        }
        inbound(Inbound::Leave { client });
    })?;
    Ok(())
}

use std::io::Read;

/// Connexion cliente (GUI, `ctl`).
pub struct Client {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
}

impl Client {
    pub fn connect() -> Result<Client> {
        let path = socket_path();
        let stream = UnixStream::connect(&path).with_context(|| format!("connecting to {}", path.display()))?;
        Ok(Client { writer: stream.try_clone()?, reader: BufReader::new(stream) })
    }

    pub fn send(&mut self, cmd: &Command) -> Result<()> {
        let mut s = serde_json::to_string(cmd)?;
        s.push('\n');
        self.writer.write_all(s.as_bytes())?;
        Ok(())
    }

    pub fn recv(&mut self) -> Result<Message> {
        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            bail!("daemon closed the connection");
        }
        Ok(serde_json::from_str(&line)?)
    }

    pub fn set_read_timeout(&self, d: Option<Duration>) -> Result<()> {
        self.reader.get_ref().set_read_timeout(d)?;
        Ok(())
    }

    /// Sépare la connexion en une moitié écriture et un flux de messages.
    pub fn split(self) -> (Sender, Receiver<Result<Message>>) {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut reader = self.reader;
        std::thread::spawn(move || {
            loop {
                let mut line = String::new();
                let msg = match reader.read_line(&mut line) {
                    Ok(0) => Err(anyhow::anyhow!("daemon closed the connection")),
                    Ok(_) => serde_json::from_str(&line).map_err(Into::into),
                    Err(e) => Err(e.into()),
                };
                let stop = msg.is_err();
                if tx.send(msg).is_err() || stop {
                    break;
                }
            }
        });
        (Sender(self.writer), rx)
    }
}

pub struct Sender(UnixStream);

impl Sender {
    pub fn send(&mut self, cmd: &Command) -> Result<()> {
        let mut s = serde_json::to_string(cmd)?;
        s.push('\n');
        self.0.write_all(s.as_bytes())?;
        Ok(())
    }
}

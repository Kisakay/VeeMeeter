//! Une seule fenêtre et un seul tray par session : chacun tient un socket
//! Unix abstrait (libéré par le noyau à la mort du processus, rien à nettoyer).
//! Le socket de la fenêtre reçoit aussi les ordres du tray (`show`, `quit`).

use std::io::{BufRead, BufReader, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::{SocketAddr, UnixListener, UnixStream};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Order {
    Show,
    Quit,
}

fn addr(role: &str) -> Option<SocketAddr> {
    SocketAddr::from_abstract_name(name(role)).ok()
}

/// Nom du socket : propre à l'utilisateur ET à sa session (dossier runtime),
/// comme le socket du daemon. Un environnement isolé (tests, captures) ne
/// réveille donc jamais la fenêtre de la vraie session.
fn name(role: &str) -> String {
    use std::hash::{Hash, Hasher};
    let uid = std::fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(0);
    let mut h = std::collections::hash_map::DefaultHasher::new();
    crate::ipc::socket_path().hash(&mut h);
    format!("veemeeter-{role}-{uid}-{:016x}", h.finish())
}

/// Réserve le rôle (`gui` ou `tray`). `None` : un autre processus le tient déjà.
pub fn claim(role: &str) -> Option<UnixListener> {
    UnixListener::bind_addr(&addr(role)?).ok()
}

pub fn running(role: &str) -> bool {
    addr(role).is_some_and(|a| UnixStream::connect_addr(&a).is_ok())
}

/// Envoie un ordre à la fenêtre ouverte. `false` : aucune fenêtre.
pub fn tell_gui(order: Order) -> bool {
    let Some(mut s) = addr("gui").and_then(|a| UnixStream::connect_addr(&a).ok()) else { return false };
    let word = match order {
        Order::Show => "show\n",
        Order::Quit => "quit\n",
    };
    s.write_all(word.as_bytes()).is_ok()
}

/// Écoute les ordres adressés à la fenêtre (thread dédié).
pub fn listen(listener: UnixListener, on_order: impl Fn(Order) + Send + 'static) {
    std::thread::Builder::new()
        .name("instance".into())
        .spawn(move || {
            for conn in listener.incoming().flatten() {
                let _ = conn.set_read_timeout(Some(Duration::from_secs(1)));
                let mut line = String::new();
                if BufReader::new(conn).read_line(&mut line).is_err() {
                    continue;
                }
                match line.trim() {
                    "show" => on_order(Order::Show),
                    "quit" => on_order(Order::Quit),
                    _ => {}
                }
            }
        })
        .expect("spawn instance thread");
}

#[cfg(test)]
mod tests {
    #[test]
    fn names_follow_the_session() {
        assert_ne!(super::name("gui"), super::name("tray"));
        assert!(super::name("gui").starts_with("veemeeter-gui-"));
    }
}

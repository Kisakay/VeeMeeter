//! Appariement des ports entre deux nœuds (logique pure).
//!
//! Règles, dans l'ordre :
//! 1. sortie mono ⇒ vers tous les canaux d'entrée ;
//! 2. entrée mono ⇒ toutes les sorties mixées dedans ;
//! 3. même position (`FL`→`FL`) ;
//! 4. aucune position commune ⇒ par ordre (`AUX0`→`FL`…).

#[derive(Clone, Copy, Debug)]
pub struct P<'a> {
    pub id: u32,
    pub channel: &'a str,
}

pub fn match_ports(outs: &[P], ins: &[P]) -> Vec<(u32, u32)> {
    if outs.is_empty() || ins.is_empty() {
        return vec![];
    }
    if outs.len() == 1 && outs[0].channel == "MONO" {
        return ins.iter().map(|i| (outs[0].id, i.id)).collect();
    }
    if ins.len() == 1 && ins[0].channel == "MONO" {
        return outs.iter().map(|o| (o.id, ins[0].id)).collect();
    }
    let by_name: Vec<(u32, u32)> =
        outs.iter().filter_map(|o| ins.iter().find(|i| i.channel == o.channel).map(|i| (o.id, i.id))).collect();
    if !by_name.is_empty() {
        return by_name;
    }
    outs.iter().zip(ins).map(|(o, i)| (o.id, i.id)).collect()
}

/// Choisit le périphérique d'un premier lancement : le défaut du système s'il
/// est utilisable, sinon le meilleur candidat. Les sources « monitor » et
/// « loopback » (son de l'ordinateur renvoyé par la carte) ne sont prises
/// qu'en dernier recours ; pour une entrée, un micro est préféré.
pub fn pick_device(candidates: &[(&str, &str)], default: Option<&str>, capture: bool) -> Option<String> {
    let looped = |name: &str, desc: &str| {
        let t = format!("{name} {desc}").to_lowercase();
        t.contains("monitor") || t.contains("loopback")
    };
    if let Some(d) = default {
        if let Some(&(n, desc)) = candidates.iter().find(|(n, _)| *n == d) {
            if !looped(n, desc) {
                return Some(n.to_string());
            }
        }
    }
    let score = |&(n, desc): &(&str, &str)| -> i32 {
        let t = format!("{n} {desc}").to_lowercase();
        let mut s = 0;
        if looped(n, desc) {
            s -= 10;
        }
        if capture && t.contains("mic") {
            s += 2;
        }
        s
    };
    // Meilleur score ; à égalité, l'ordre du graphe (premier trouvé).
    let mut best: Option<(i32, &str)> = None;
    for c in candidates {
        let sc = score(c);
        if best.is_none_or(|(b, _)| sc > b) {
            best = Some((sc, c.0));
        }
    }
    best.map(|(_, n)| n.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ps<'a>(base: u32, ch: &[&'a str]) -> Vec<P<'a>> {
        ch.iter().enumerate().map(|(i, c)| P { id: base + i as u32, channel: c }).collect()
    }

    #[test]
    fn stereo_to_stereo() {
        assert_eq!(match_ports(&ps(1, &["FL", "FR"]), &ps(10, &["FR", "FL"])), [(1, 11), (2, 10)]);
    }

    #[test]
    fn mono_fans_out_and_in() {
        assert_eq!(match_ports(&ps(1, &["MONO"]), &ps(10, &["FL", "FR"])), [(1, 10), (1, 11)]);
        assert_eq!(match_ports(&ps(1, &["FL", "FR"]), &ps(10, &["MONO"])), [(1, 10), (2, 10)]);
    }

    #[test]
    fn surround_keeps_matching_positions() {
        let six = ps(10, &["FL", "FR", "FC", "LFE", "RL", "RR"]);
        assert_eq!(match_ports(&ps(1, &["FL", "FR"]), &six), [(1, 10), (2, 11)]);
        assert_eq!(match_ports(&six, &ps(1, &["FL", "FR"])), [(10, 1), (11, 2)]);
    }

    #[test]
    fn first_run_skips_loopback_sources() {
        // Cas réel : Scarlett 2i2, l'entrée loopback est annoncée avant le micro.
        let c = [
            ("alsa_input.HiFi__Line2__source", "Scarlett 2i2 4th Gen Monitor Input 3+4 (stereo)"),
            ("alsa_input.HiFi__Mic1__source", "Scarlett 2i2 4th Gen Input 1 Mic/Inst/Line"),
            ("alsa_input.HiFi__Mic2__source", "Scarlett 2i2 4th Gen Input 2 Mic/Inst/Line"),
        ];
        assert_eq!(pick_device(&c, None, true).as_deref(), Some("alsa_input.HiFi__Mic1__source"));
        // Le défaut du système gagne… sauf si c'est un loopback.
        assert_eq!(
            pick_device(&c, Some("alsa_input.HiFi__Mic2__source"), true).as_deref(),
            Some("alsa_input.HiFi__Mic2__source")
        );
        assert_eq!(
            pick_device(&c, Some("alsa_input.HiFi__Line2__source"), true).as_deref(),
            Some("alsa_input.HiFi__Mic1__source")
        );
        // Rien d'autre qu'un loopback : on le prend quand même.
        assert_eq!(pick_device(&c[..1], None, true).as_deref(), Some("alsa_input.HiFi__Line2__source"));
        assert_eq!(pick_device(&[], None, true), None);
    }

    #[test]
    fn falls_back_to_order() {
        assert_eq!(match_ports(&ps(1, &["AUX0", "AUX1"]), &ps(10, &["FL", "FR"])), [(1, 10), (2, 11)]);
        assert!(match_ports(&[], &ps(10, &["FL"])).is_empty());
    }
}

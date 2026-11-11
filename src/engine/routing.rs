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
    fn falls_back_to_order() {
        assert_eq!(match_ports(&ps(1, &["AUX0", "AUX1"]), &ps(10, &["FL", "FR"])), [(1, 10), (2, 11)]);
        assert!(match_ports(&[], &ps(10, &["FL"])).is_empty());
    }
}

//! Miroir local du graphe PipeWire, alimenté par le registre.

use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct GNode {
    pub id: u32,
    pub serial: Option<u64>,
    pub name: String,
    pub class: String,
    pub description: String,
    pub app_name: Option<String>,
    pub media_name: Option<String>,
    pub icon: Option<String>,
    pub channels: Vec<String>,
    /// Flux de mesure (`stream.monitor`) : VU-mètres de pavucontrol, du bureau, les nôtres.
    pub monitor: bool,
}

#[derive(Clone, Debug)]
pub struct GPort {
    pub id: u32,
    pub node: u32,
    pub output: bool,
    pub monitor: bool,
    pub channel: String,
    /// Index du port dans le nœud (`port.id`), pour l'appariement par ordre.
    pub index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GLink {
    pub id: u32,
    pub out_node: u32,
    pub out_port: u32,
    pub in_node: u32,
    pub in_port: u32,
}

#[derive(Default)]
pub struct Graph {
    pub nodes: BTreeMap<u32, GNode>,
    pub ports: BTreeMap<u32, GPort>,
    pub links: BTreeMap<u32, GLink>,
}

pub type Props<'a> = &'a dyn Fn(&str) -> Option<String>;

impl Graph {
    pub fn add_node(&mut self, id: u32, p: Props) {
        let channels = p("audio.position")
            .map(|s| {
                s.trim_matches(|c| c == '[' || c == ']')
                    .split([',', ' '])
                    .filter(|c| !c.is_empty())
                    .map(|c| c.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();
        let name = p("node.name").unwrap_or_default();
        self.nodes.insert(
            id,
            GNode {
                id,
                serial: p("object.serial").and_then(|s| s.parse().ok()),
                description: p("node.description").or_else(|| p("node.nick")).unwrap_or_else(|| name.clone()),
                name,
                class: p("media.class").unwrap_or_default(),
                app_name: p("application.name"),
                media_name: p("media.name"),
                icon: p("application.icon-name").or_else(|| p("application.process.binary")),
                channels,
                monitor: p("stream.monitor").as_deref() == Some("true"),
            },
        );
    }

    pub fn add_port(&mut self, id: u32, p: Props) {
        let Some(node) = p("node.id").and_then(|s| s.parse().ok()) else { return };
        let output = p("port.direction").as_deref() == Some("out");
        let name = p("port.name").unwrap_or_default();
        let channel = p("audio.channel")
            .unwrap_or_else(|| name.rsplit('_').next().unwrap_or("").to_string())
            .to_ascii_uppercase();
        self.ports.insert(
            id,
            GPort {
                id,
                node,
                output,
                monitor: p("port.monitor").as_deref() == Some("true"),
                channel,
                index: p("port.id").and_then(|s| s.parse().ok()).unwrap_or(id),
            },
        );
    }

    pub fn add_link(&mut self, id: u32, p: Props) {
        let n = |k: &str| p(k).and_then(|s| s.parse().ok());
        if let (Some(out_node), Some(out_port), Some(in_node), Some(in_port)) =
            (n("link.output.node"), n("link.output.port"), n("link.input.node"), n("link.input.port"))
        {
            self.links.insert(id, GLink { id, out_node, out_port, in_node, in_port });
        }
    }

    pub fn remove(&mut self, id: u32) -> bool {
        self.nodes.remove(&id).is_some() || self.ports.remove(&id).is_some() || self.links.remove(&id).is_some()
    }

    pub fn node_by_name(&self, name: &str) -> Option<&GNode> {
        self.nodes.values().find(|n| n.name == name)
    }

    /// Ports d'un nœud dans un sens, triés par index. Les ports monitor des
    /// sinks matériels sont exclus des sorties (seuls les nôtres servent).
    pub fn ports_of(&self, node: u32, output: bool) -> Vec<&GPort> {
        let mut v: Vec<&GPort> = self.ports.values().filter(|p| p.node == node && p.output == output).collect();
        v.sort_by_key(|p| (p.index, p.id));
        v
    }

    pub fn has_link(&self, out_port: u32, in_port: u32) -> bool {
        self.links.values().any(|l| l.out_port == out_port && l.in_port == in_port)
    }
}

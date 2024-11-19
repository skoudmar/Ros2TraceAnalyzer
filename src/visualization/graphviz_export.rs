use std::collections::HashMap;
use std::fmt::Display;

use derive_more::derive::Display;

#[derive(Debug, Clone, Default)]
pub struct Graph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    clusters: Vec<GraphCluster>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, name: &str, id: usize) -> &mut GraphNode {
        self.nodes.push(GraphNode::new(id, name));
        self.nodes.iter_mut().last().unwrap()
    }

    pub fn add_cluster(&mut self, name: &str, node_ids: Vec<usize>) {
        let id = self.clusters.len();
        let cluster = GraphCluster::new(id, name, node_ids);
        self.clusters.push(cluster);
    }

    pub fn add_edge(&mut self, source: usize, target: usize, label: &str) -> &mut GraphEdge {
        self.edges.push(GraphEdge::new(source, target, label));
        self.edges.iter_mut().last().unwrap()
    }
}

impl Display for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "digraph {{")?;
        for node in &self.nodes {
            writeln!(f, "\t{node}")?;
        }
        for edge in &self.edges {
            writeln!(f, "\t{edge}")?;
        }
        for cluster in &self.clusters {
            writeln!(f, "{cluster}")?;
        }
        writeln!(f, "}}")
    }
}

#[derive(Debug, Clone)]
pub struct GraphCluster {
    id: usize,
    node_ids: Vec<usize>,
    label: String,
}

impl GraphCluster {
    pub fn new(id: usize, label: &str, node_ids: Vec<usize>) -> Self {
        Self {
            id,
            node_ids,
            label: escape_string(label),
        }
    }
}

impl Display for GraphCluster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tsubgraph cluster_{} {{", self.id)?;
        writeln!(f, "\t\tlabel=\"{}\"", self.label)?;
        writeln!(f, "\t\tgraph[style=\"dotted\"]")?;

        write!(f, "\t\t")?;
        for node_id in &self.node_ids {
            write!(f, "{node_id}; ")?;
        }
        writeln!(f, "\n\t}}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Display)]
pub enum NodeShape {
    #[default]
    #[display("box")]
    Box,

    #[display("ellipse")]
    Ellipse,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    id: usize,
    name: String,
    shape: NodeShape,
    attributes: HashMap<String, String>,
}

impl GraphNode {
    fn new(id: usize, name: &str) -> Self {
        Self {
            id,
            name: escape_string(name),
            shape: NodeShape::default(),
            attributes: HashMap::new(),
        }
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = escape_string(name);
    }

    pub fn set_shape(&mut self, shape: NodeShape) {
        self.shape = shape;
    }

    pub fn set_attribute(&mut self, key: &str, value: &str) {
        assert!(
            key != "label",
            "Cannot set label attribute here, Use set_name instead!"
        );
        assert!(
            key != "shape",
            "Cannot set shape attribute here. Use set_shape instead!"
        );
        self.attributes.insert(key.to_owned(), escape_string(value));
    }
}

impl Display for GraphNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [label=\"{}\", shape={}",
            self.id, self.name, self.shape
        )?;
        for (key, value) in &self.attributes {
            write!(f, ", {key}=\"{value}\"")?;
        }
        write!(f, "]")
    }
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    src: usize,
    dst: usize,
    label: String,
    attributes: HashMap<&'static str, String>,
}

impl GraphEdge {
    fn new(src: usize, dst: usize, label: &str) -> Self {
        Self {
            src,
            dst,
            label: escape_string(label),
            attributes: HashMap::new(),
        }
    }

    pub fn set_attribute(&mut self, key: &'static str, value: &str) {
        assert!(key != "label", "Cannot set label attribute");
        self.attributes.insert(key, escape_string(value));
    }
}

impl Display for GraphEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {} [label=\"{}\"", self.src, self.dst, self.label)?;
        for (key, value) in &self.attributes {
            write!(f, ", {key}=\"{value}\"")?;
        }
        write!(f, "]")
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

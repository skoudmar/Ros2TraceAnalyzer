use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;

use derive_more::derive::Display;

#[derive(Debug, Clone, Default)]
pub struct Graph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    clusters: Vec<GraphCluster>,
    attributes: Attributes,
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

    pub fn set_attribute(&mut self, key: &'static str, value: &str) {
        self.attributes.set(key, value);
    }
}

impl Display for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "digraph {{")?;
        writeln!(f, "\tgraph {}", self.attributes)?;

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
        writeln!(f, "\t\tgraph [style=\"dotted\"]")?;

        write!(f, "\t\t")?;
        for node_id in &self.node_ids {
            write!(f, "{node_id}; ")?;
        }
        writeln!(f, "\n\t}}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Display)]
#[display("{}", std::convert::Into::<&'static str>::into(*self))]
pub enum NodeShape {
    #[default]
    Box,
    Ellipse,
}

impl From<NodeShape> for &'static str {
    fn from(shape: NodeShape) -> &'static str {
        match shape {
            NodeShape::Box => "box",
            NodeShape::Ellipse => "ellipse",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    id: usize,
    attributes: Attributes,
}

impl GraphNode {
    fn new(id: usize, name: &str) -> Self {
        let mut attributes = Attributes::new();
        attributes.set("label", name);
        attributes.set("shape", NodeShape::default().into());
        Self { id, attributes }
    }

    pub fn set_shape(&mut self, shape: NodeShape) {
        self.attributes.set("shape", shape.into());
    }

    pub fn set_attribute(&mut self, key: impl Into<Cow<'static, str>>, value: &str) {
        self.attributes.set(key, value);
    }
}

impl Display for GraphNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.attributes)
    }
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    src: usize,
    dst: usize,
    attributes: Attributes,
}

impl GraphEdge {
    fn new(src: usize, dst: usize, label: &str) -> Self {
        let mut attributes = Attributes::new();
        attributes.set("label", label);
        Self {
            src,
            dst,
            attributes,
        }
    }

    pub fn set_attribute(&mut self, key: impl Into<Cow<'static, str>>, value: &str) {
        self.attributes.set(key, value);
    }
}

impl Display for GraphEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {} {}", self.src, self.dst, self.attributes)
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[derive(Debug, Clone, Default)]
struct Attributes {
    attributes: HashMap<Cow<'static, str>, String>,
}

impl Attributes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<Cow<'static, str>>, value: &str) {
        self.attributes.insert(key.into(), value.to_string());
    }
}

impl Display for Attributes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.attributes.iter();
        // Write value with Debug to escape special characters
        write!(f, "[")?;
        if let Some((key, value)) = iter.next() {
            write!(f, "{key}={value:?}",)?;
        }
        for (key, value) in iter {
            write!(f, ", {key}={value:?}")?;
        }
        write!(f, "]")
    }
}

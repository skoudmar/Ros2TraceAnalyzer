use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use graph::Graph;

use crate::model::{Callback, CallbackType};
use crate::processed_events::{ros2, Event, FullEvent};

use super::{ArcMutWrapper, EventAnalysis, PublicationInCallback};

pub mod graph {
    use crate::model::display::DisplayCallbackSummary;
    use crate::model::Callback;
    use crate::visualization;
    use crate::visualization::graphviz_export;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, Default)]
    pub struct Graph {
        nodes: Vec<Node>,
        edges: Vec<(usize, usize)>,

        sources: Vec<usize>,
    }

    impl Graph {
        pub(super) const fn new(
            nodes: Vec<Node>,
            edges: Vec<(usize, usize)>,
            sources: Vec<usize>,
        ) -> Self {
            Self {
                nodes,
                edges,
                sources,
            }
        }

        #[inline]
        pub fn nodes(&self) -> &[Node] {
            &self.nodes
        }

        #[inline]
        pub fn edges(&self) -> &[(usize, usize)] {
            &self.edges
        }

        #[inline]
        pub fn sources(&self) -> &[usize] {
            &self.sources
        }

        pub fn print_graph(&self) {
            println!("Graph:");
            println!("  Nodes:");
            for (i, node) in self.nodes().iter().enumerate() {
                let callback = node.callback().lock().unwrap();
                println!("    [{i:4}] Callback{}", DisplayCallbackSummary(&callback));
            }

            println!("  Edges:");
            for (src, dst) in self.edges() {
                println!("    {src} -> {dst}");
            }

            println!("Sources: {:?}", self.sources());
        }

        pub fn as_dot(&self) -> graphviz_export::Graph {
            let mut graph = graphviz_export::Graph::new();
            graph.set_attribute("rankdir", "LR");

            for (i, node) in self.nodes().iter().enumerate() {
                let callback = node.callback().lock().unwrap();
                let label = DisplayCallbackSummary(&callback).to_string();
                let label = label.replace(", ", "\n");
                let label = &label[1..label.len() - 1];
                let node = graph.add_node(&format!("Callback\n{label}"), i);
                node.set_shape(graphviz_export::NodeShape::Ellipse);
            }

            for (src, dst) in self.edges() {
                let src_idx = *src;
                let dst_idx = *dst;
                graph.add_edge(src_idx, dst_idx, "");
            }

            graph
        }
    }

    #[derive(Debug, Clone)]
    pub struct Node {
        callback: Arc<Mutex<Callback>>,
    }

    impl Node {
        pub(super) const fn new(callback: Arc<Mutex<Callback>>) -> Self {
            Self { callback }
        }

        pub const fn callback(&self) -> &Arc<Mutex<Callback>> {
            &self.callback
        }
    }
}

#[derive(Debug, Default)]
pub struct CallbackDependency {
    timer_driven_callbacks: Vec<ArcMutWrapper<Callback>>,
    message_driven_callbacks: Vec<ArcMutWrapper<Callback>>,
    publication_in_callback: PublicationInCallback,
    graph: Option<Box<Graph>>,
}

impl CallbackDependency {
    pub fn new() -> Self {
        Self::default()
    }

    fn add_callback(&mut self, callback: &Arc<Mutex<Callback>>) {
        let callback_arc = callback.clone();
        let callback = callback.lock().unwrap();

        match callback.get_type().unwrap() {
            CallbackType::Timer => {
                self.timer_driven_callbacks.push(callback_arc.into());
            }
            CallbackType::Subscription => {
                self.message_driven_callbacks.push(callback_arc.into());
            }
            CallbackType::Service => {
                // Ignore service callbacks for now
                // TODO: Handle service callbacks
            }
        }
    }

    fn construct_callback_graph(&mut self) {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let mut callback_to_node = HashMap::new();
        let mut topic_to_nodes = HashMap::<_, Vec<_>>::new();

        let mut sources = Vec::new();

        for callback in &self.timer_driven_callbacks {
            let node = graph::Node::new(callback.0.clone());

            let id = nodes.len();
            nodes.push(node);
            sources.push(id);
            callback_to_node.insert(callback.clone(), id);
        }

        for callback in &self.message_driven_callbacks {
            let node = graph::Node::new(callback.0.clone());

            let id = nodes.len();
            nodes.push(node);
            callback_to_node.insert(callback.clone(), id);

            let callback = callback.0.lock().unwrap();
            let subscriber = callback.get_caller().unwrap().unwrap_subscription_ref();

            let subscriber = subscriber.get_arc().expect("Subscriber should be alive");
            let subscriber = subscriber.lock().unwrap();
            let topic = subscriber.get_topic().unwrap().to_owned();

            topic_to_nodes.entry(topic).or_default().push(id);
        }

        for (publisher, callback) in self.publication_in_callback.get_dependency() {
            let publisher = publisher.0.lock().unwrap();
            let topic = publisher.get_topic().unwrap().to_owned();

            if let Some(nodes) = topic_to_nodes.get(&topic) {
                for &node in nodes {
                    edges.push((callback_to_node[callback], node));
                }
            }
        }

        let graph = Graph::new(nodes, edges, sources);

        self.graph = Some(Box::new(graph));
    }

    pub fn get_graph(&self) -> Option<&Graph> {
        self.graph.as_deref()
    }

    pub(crate) fn get_publication_in_callback_analysis(&self) -> &PublicationInCallback {
        &self.publication_in_callback
    }
}

impl EventAnalysis for CallbackDependency {
    fn initialize(&mut self) {
        // Clear all data
        *self = Self::default();
    }

    fn process_event(&mut self, full_event: &FullEvent) {
        self.publication_in_callback.process_event(full_event);
        if let Event::Ros2(ros2::Event::RclcppCallbackRegister(event)) = &full_event.event {
            self.add_callback(&event.callback);
        }
    }

    fn finalize(&mut self) {
        self.construct_callback_graph();
    }
}

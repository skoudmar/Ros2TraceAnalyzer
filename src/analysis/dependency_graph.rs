use std::collections::{HashMap, HashSet};
use std::ops::Not;
use std::sync::{Arc, Mutex};

use crate::analysis::utils::DisplayDurationStats;
use crate::events_common::Context;
use crate::model::{
    self, Callback, CallbackCaller, CallbackInstance, CallbackTrigger, Publisher, Service,
    Subscriber, Time, Timer,
};
use crate::processed_events::{r2r, ros2, Event, FullEvent};
use crate::statistics::Sorted;
use crate::utils::{DisplayDuration, Known};
use crate::visualization::graphviz_export::{self, NodeShape};
use crate::visualization::COLOR_GRADIENT;

use super::{ArcMutWrapper, EventAnalysis};

const LATENCY_INVALID: i64 = i64::MAX;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ThreadId {
    vtid: u32,
    hostname: String,
}

impl From<&Context> for ThreadId {
    fn from(context: &Context) -> Self {
        Self {
            vtid: context.vtid(),
            hostname: context.hostname().to_string(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct DependencyGraph {
    ros_nodes: Vec<Arc<Mutex<model::Node>>>,

    edges: HashMap<Edge, EdgeData>,

    publisher_nodes: HashMap<ArcMutWrapper<Publisher>, PublisherNode>,
    subscriber_nodes: HashMap<ArcMutWrapper<Subscriber>, SubscriberNode>,
    timer_nodes: HashMap<ArcMutWrapper<Timer>, TimerNode>,
    callback_nodes: HashMap<ArcMutWrapper<Callback>, CallbackNode>,

    last_spin_wake_up_time_for_node: HashMap<ArcMutWrapper<model::Node>, Time>,
    running_callbacks: HashMap<ThreadId, Arc<Mutex<CallbackInstance>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Node {
    Publisher(ArcMutWrapper<Publisher>),
    Subscriber(ArcMutWrapper<Subscriber>),
    Service(ArcMutWrapper<Service>),
    Timer(ArcMutWrapper<Timer>),
    Callback(ArcMutWrapper<Callback>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct PublisherNode {
    /// Time between two consecutive publications
    publication_delay: Vec<i64>,

    /// Time of the last publication
    last_publication: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SubscriberNode {
    /// Time between two consecutive take events
    take_delay: Vec<i64>,

    /// Time of the last take event
    last_take: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TimerNode {
    /// Time between two consecutive timer activations
    activation_delay: Vec<i64>,

    /// Last activation time
    last_activation: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct CallbackNode {
    /// Time between two consecutive callback activations. (c1.start) -> (c2.start)
    activation_delay: Vec<i64>,

    /// Duration of the callback execution. (c1.start) -> (c1.end)
    durations: Vec<i64>,

    /// Last activation time
    last_activation: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Edge {
    PublicationInCallback(ArcMutWrapper<Publisher>, ArcMutWrapper<Callback>),
    SubscriberCallbackInvocation(ArcMutWrapper<Subscriber>, ArcMutWrapper<Callback>),
    ServiceCallbackInvocation(ArcMutWrapper<Service>, ArcMutWrapper<Callback>),
    TimerCallbackInvocation(ArcMutWrapper<Timer>, ArcMutWrapper<Callback>),
    PublisherSubscriberCommunication(ArcMutWrapper<Publisher>, ArcMutWrapper<Subscriber>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EdgeType {
    PublisherSubscriberCommunication,
    SubscriberCallbackInvocation,
    TimerCallbackInvocation,
    ServiceCallbackInvocation,
    PublicationInCallback,
}

impl Edge {
    fn source(&self) -> Node {
        match self {
            Edge::PublicationInCallback(_publisher, callback) => Node::Callback(callback.clone()),
            Edge::SubscriberCallbackInvocation(subscriber, _callback) => {
                Node::Subscriber(subscriber.clone())
            }
            Edge::ServiceCallbackInvocation(service, _callback) => Node::Service(service.clone()),
            Edge::TimerCallbackInvocation(timer, _callback) => Node::Timer(timer.clone()),
            Edge::PublisherSubscriberCommunication(publisher, _subscriber) => {
                Node::Publisher(publisher.clone())
            }
        }
    }

    fn target(&self) -> Node {
        match self {
            Edge::PublicationInCallback(publisher, _callback) => Node::Publisher(publisher.clone()),
            Edge::SubscriberCallbackInvocation(_subscriber, callback) => {
                Node::Callback(callback.clone())
            }
            Edge::ServiceCallbackInvocation(_service, callback) => Node::Callback(callback.clone()),
            Edge::TimerCallbackInvocation(_timer, callback) => Node::Callback(callback.clone()),
            Edge::PublisherSubscriberCommunication(_publisher, subscriber) => {
                Node::Subscriber(subscriber.clone())
            }
        }
    }

    pub fn as_type(&self) -> EdgeType {
        match self {
            Edge::PublicationInCallback(_, _) => EdgeType::PublicationInCallback,
            Edge::SubscriberCallbackInvocation(_, _) => EdgeType::SubscriberCallbackInvocation,
            Edge::ServiceCallbackInvocation(_, _) => EdgeType::ServiceCallbackInvocation,
            Edge::TimerCallbackInvocation(_, _) => EdgeType::TimerCallbackInvocation,
            Edge::PublisherSubscriberCommunication(_, _) => {
                EdgeType::PublisherSubscriberCommunication
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct EdgeData {
    activation_delay: Vec<i64>,
    latencies: Vec<i64>,
    last_activation: Option<Time>,
}

// Public API
impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn display_as_dot(&self) -> DisplayAsDot {
        DisplayAsDot::new(self)
    }
}

// Calculations
impl DependencyGraph {
    fn add_ros_node(&mut self, node: Arc<Mutex<model::Node>>) {
        self.ros_nodes.push(node);
    }

    fn process_timer_invocation(&mut self, timer: &Arc<Mutex<Timer>>, event_time: Time) {
        let timer_node = self.timer_nodes.entry(timer.clone().into()).or_default();
        if let Some(previous_activation) = timer_node.last_activation.replace(event_time) {
            let activation_delay =
                event_time.timestamp_nanos() - previous_activation.timestamp_nanos();
            timer_node.activation_delay.push(activation_delay);
        } else {
            debug_assert!(timer_node.activation_delay.is_empty());
        }
    }

    fn process_edge_to_callback(
        &mut self,
        callback_arc: ArcMutWrapper<Callback>,
        trigger: &CallbackTrigger,
        event_time: Time,
    ) {
        match trigger {
            CallbackTrigger::SubscriptionMessage(msg) => {
                let message = msg.lock().unwrap();
                let subscriber = message.get_subscriber().unwrap();
                let edge = Edge::SubscriberCallbackInvocation(subscriber.into(), callback_arc);
                let edge_data = self.edges.entry(edge).or_default();

                if let Some(previous_activation) = edge_data.last_activation.replace(event_time) {
                    debug_assert_eq!(
                        edge_data.activation_delay.len() + 1,
                        edge_data.latencies.len()
                    );

                    let activation_delay =
                        event_time.timestamp_nanos() - previous_activation.timestamp_nanos();
                    edge_data.activation_delay.push(activation_delay);
                } else {
                    debug_assert!(edge_data.latencies.is_empty());
                    debug_assert!(edge_data.activation_delay.is_empty());
                }

                let receive_time = message
                    .get_rmw_receive_time()
                    .expect("RMW receive time should be known");
                let latency = event_time.timestamp_nanos() - receive_time.timestamp_nanos();
                edge_data.latencies.push(latency);
            }
            CallbackTrigger::Timer(timer) => {
                // Timer and Timer callback have the same data because the timer invocation is the
                // callback invocation.
                self.process_timer_invocation(timer, event_time);

                let edge = Edge::TimerCallbackInvocation(timer.clone().into(), callback_arc);
                let edge_data = self.edges.entry(edge).or_default();

                if let Some(previous_activation) = edge_data.last_activation.replace(event_time) {
                    debug_assert_eq!(
                        edge_data.activation_delay.len() + 1,
                        edge_data.latencies.len()
                    );

                    let activation_delay =
                        event_time.timestamp_nanos() - previous_activation.timestamp_nanos();
                    edge_data.activation_delay.push(activation_delay);
                } else {
                    debug_assert!(edge_data.latencies.is_empty());
                    debug_assert!(edge_data.activation_delay.is_empty());
                }

                // Since the timer and the callback invocations are the same, the latency would be 0.
                // We change the latency to the time between the last spin wake up and the callback.
                let timer = timer.lock().unwrap();
                let node_arc = timer
                    .get_node()
                    .expect("Timer should be associated with a node when invoked.")
                    .get_arc()
                    .expect("Node should be alive.");
                let latency = self
                    .last_spin_wake_up_time_for_node
                    .get(&node_arc.into())
                    .map_or(LATENCY_INVALID, |wake_up_time| {
                        event_time.timestamp_nanos() - wake_up_time.timestamp_nanos()
                    });

                edge_data.latencies.push(latency);
            }
            CallbackTrigger::Service(service_arc) => {
                let edge =
                    Edge::ServiceCallbackInvocation(service_arc.clone().into(), callback_arc);
                let edge_data = self.edges.entry(edge).or_default();

                if let Some(previous_activation) = edge_data.last_activation.replace(event_time) {
                    debug_assert_eq!(
                        edge_data.activation_delay.len() + 1,
                        edge_data.latencies.len()
                    );

                    let activation_delay =
                        event_time.timestamp_nanos() - previous_activation.timestamp_nanos();
                    edge_data.activation_delay.push(activation_delay);
                } else {
                    debug_assert!(edge_data.latencies.is_empty());
                    debug_assert!(edge_data.activation_delay.is_empty());
                }

                let service = service_arc.lock().unwrap();
                let node_arc = service
                    .get_node()
                    .expect("Service should be associated with a node when invoked.")
                    .get_arc()
                    .expect("Node should be alive.");
                let latency = self
                    .last_spin_wake_up_time_for_node
                    .get(&node_arc.into())
                    .map_or(LATENCY_INVALID, |wake_up_time| {
                        event_time.timestamp_nanos() - wake_up_time.timestamp_nanos()
                    });

                edge_data.latencies.push(latency);
            }
        }
    }

    fn process_callback_start(
        &mut self,
        event: &ros2::CallbackStart,
        event_time: Time,
        context: &Context,
    ) {
        self.running_callbacks
            .insert(context.into(), event.callback.clone())
            .inspect(|old| {
                panic!(
                    "Callback {old:?} is already running on vtid {} on host {}",
                    context.vtid(),
                    context.hostname()
                );
            });

        let callback_instance = event.callback.lock().unwrap();
        let callback = callback_instance.get_callback();

        let callback_node = self
            .callback_nodes
            .entry(callback.clone().into())
            .or_default();
        let previous_activation = callback_node.last_activation.replace(event_time);
        if let Some(previous_activation) = previous_activation {
            debug_assert_eq!(event_time, callback_instance.get_start_time());
            let activation_delay =
                event_time.timestamp_nanos() - previous_activation.timestamp_nanos();
            callback_node.activation_delay.push(activation_delay);
        }

        self.process_edge_to_callback(callback.into(), callback_instance.get_trigger(), event_time);
    }

    fn process_callback_end(
        &mut self,
        event: &ros2::CallbackEnd,
        event_time: Time,
        context: &Context,
    ) {
        self.running_callbacks
            .remove(&context.into())
            .and_then(|callback| {
                Arc::ptr_eq(&callback, &event.callback)
                    .not()
                    .then_some(callback)
            })
            .inspect(|old| {
                panic!(
                    "Callback {old:?} is running on vtid {} on host {} instead of expected {:?}",
                    context.vtid(),
                    context.hostname(),
                    event.callback
                );
            });

        let callback_instance = event.callback.lock().unwrap();
        let callback = callback_instance.get_callback();

        let callback_node = self.callback_nodes.get_mut(&callback.into()).unwrap();
        let start_time = callback_node
            .last_activation
            .expect("Last activation should be known. It is set in start event.");
        let end_time = event_time;
        let duration = end_time.timestamp_nanos() - start_time.timestamp_nanos();

        if cfg!(debug_assertions) {
            let end_time_instance = callback_instance
                .get_end_time()
                .expect("End time should be set in end event.");
            let start_time_instance = callback_instance.get_start_time();

            debug_assert_eq!(start_time_instance, start_time);
            debug_assert_eq!(end_time_instance, end_time);
        }

        callback_node.durations.push(duration);
    }

    fn process_rmw_take(&mut self, event: &ros2::RmwTake, event_time: Time) {
        if !event.taken {
            // Only process taken messages
            return;
        }
        let message = event.message.lock().unwrap();
        let subscriber_arc = message.get_subscriber().unwrap();
        let subscriber_node = self
            .subscriber_nodes
            .entry(subscriber_arc.clone().into())
            .or_default();
        if let Some(previous_take) = subscriber_node.last_take.replace(event_time) {
            let take_delay = event_time.timestamp_nanos() - previous_take.timestamp_nanos();
            subscriber_node.take_delay.push(take_delay);
        } else {
            debug_assert!(subscriber_node.take_delay.is_empty());
        }

        let Some(publication_message) = message.get_publication_message() else {
            // Ignore messages that cannot be associated with a publication message
            return;
        };

        let publication_message = publication_message.lock().unwrap();
        let publisher_arc = publication_message
            .get_publisher()
            .expect("Publisher should be known.");
        let edge =
            Edge::PublisherSubscriberCommunication(publisher_arc.into(), subscriber_arc.into());
        let edge_data = self.edges.entry(edge).or_default();

        if let Some(previous_activation) = edge_data.last_activation.replace(event_time) {
            debug_assert_eq!(
                edge_data.activation_delay.len() + 1,
                edge_data.latencies.len()
            );

            let activation_delay =
                event_time.timestamp_nanos() - previous_activation.timestamp_nanos();
            edge_data.activation_delay.push(activation_delay);
        } else {
            debug_assert!(edge_data.latencies.is_empty());
            debug_assert!(edge_data.activation_delay.is_empty());
        }

        let receive_time = event_time;
        let latency = receive_time.timestamp_nanos()
            - publication_message
                .get_publication_time()
                .expect("Publication time should be known on published messages")
                .timestamp_nanos();

        edge_data.latencies.push(latency);
    }

    fn process_publication(
        &mut self,
        event: &ros2::RclPublish,
        event_time: Time,
        context: &Context,
    ) {
        let publication = event.message.lock().unwrap();
        let publisher_arc = publication.get_publisher().unwrap();

        let publisher_node = self
            .publisher_nodes
            .entry(publisher_arc.clone().into())
            .or_default();

        if let Some(previous_publication) = publisher_node.last_publication.replace(event_time) {
            let publication_delay =
                event_time.timestamp_nanos() - previous_publication.timestamp_nanos();
            publisher_node.publication_delay.push(publication_delay);
        } else {
            debug_assert!(publisher_node.publication_delay.is_empty());
        }

        if let Some(callback_instance_arc) = self.running_callbacks.get(&context.into()) {
            let callback_instance = callback_instance_arc.lock().unwrap();
            let callback_arc = callback_instance.get_callback();
            let edge = Edge::PublicationInCallback(publisher_arc.into(), callback_arc.into());
            let edge_data = self.edges.entry(edge).or_default();

            if let Some(previous_activation) = edge_data.last_activation.replace(event_time) {
                debug_assert_eq!(
                    edge_data.activation_delay.len() + 1,
                    edge_data.latencies.len()
                );

                let activation_delay =
                    event_time.timestamp_nanos() - previous_activation.timestamp_nanos();
                edge_data.activation_delay.push(activation_delay);
            } else {
                debug_assert!(edge_data.latencies.is_empty());
                debug_assert!(edge_data.activation_delay.is_empty());
            }

            let latency =
                event_time.timestamp_nanos() - callback_instance.get_start_time().timestamp_nanos();
            edge_data.latencies.push(latency);
        }
    }
}

impl EventAnalysis for DependencyGraph {
    fn initialize(&mut self) {
        *self = Self::default();
    }

    fn process_event(&mut self, full_event: &FullEvent) {
        let event_time = full_event.time;

        match &full_event.event {
            Event::Ros2(ros2::Event::RclNodeInit(event)) => {
                self.add_ros_node(event.node.clone());
            }
            Event::Ros2(ros2::Event::CallbackStart(event)) => {
                self.process_callback_start(event, event_time, &full_event.context);
            }
            Event::Ros2(ros2::Event::CallbackEnd(event)) => {
                self.process_callback_end(event, event_time, &full_event.context);
            }

            Event::Ros2(ros2::Event::RmwTake(event)) => {
                self.process_rmw_take(event, event_time);
            }

            Event::Ros2(ros2::Event::RclPublish(event)) => {
                self.process_publication(event, event_time, &full_event.context);
            }

            Event::R2r(r2r::Event::SpinWake(event)) => {
                self.last_spin_wake_up_time_for_node
                    .insert(event.node.clone().into(), event_time);
            }
            Event::R2r(r2r::Event::SpinEnd(event)) => {
                debug_assert!(
                    self.last_spin_wake_up_time_for_node
                        .contains_key(&event.node.clone().into()),
                    "Missing spin wake up event."
                );
            }

            _ => {}
        }
    }

    fn finalize(&mut self) {
        self.running_callbacks.clear();
    }
}

struct EdgeWeightStats {
    subscriber_to_callback: (i64, i64),
    timer_to_callback: (i64, i64),
    callback_to_publisher: (i64, i64),
}

impl EdgeWeightStats {
    fn update_subscriber_to_callback(&mut self, latency: i64) {
        self.subscriber_to_callback.0 = self.subscriber_to_callback.0.min(latency);
        self.subscriber_to_callback.1 = self.subscriber_to_callback.1.max(latency);
    }

    fn update_timer_to_callback(&mut self, latency: i64) {
        self.timer_to_callback.0 = self.timer_to_callback.0.min(latency);
        self.timer_to_callback.1 = self.timer_to_callback.1.max(latency);
    }

    fn update_callback_to_publisher(&mut self, latency: i64) {
        self.callback_to_publisher.0 = self.callback_to_publisher.0.min(latency);
        self.callback_to_publisher.1 = self.callback_to_publisher.1.max(latency);
    }

    fn validate_range(range: (i64, i64)) -> Option<(i64, i64)> {
        if range.0 == i64::MAX && range.1 == i64::MIN {
            None
        } else {
            Some(range)
        }
    }

    fn range_for_type(&self, typ: EdgeType) -> Option<(i64, i64)> {
        match typ {
            EdgeType::SubscriberCallbackInvocation => {
                Self::validate_range(self.subscriber_to_callback)
            }
            EdgeType::TimerCallbackInvocation => Self::validate_range(self.timer_to_callback),
            EdgeType::PublicationInCallback => Self::validate_range(self.callback_to_publisher),
            EdgeType::ServiceCallbackInvocation | EdgeType::PublisherSubscriberCommunication => {
                None
            }
        }
    }
}

impl Default for EdgeWeightStats {
    fn default() -> Self {
        Self {
            subscriber_to_callback: (i64::MAX, i64::MIN),
            timer_to_callback: (i64::MAX, i64::MIN),
            callback_to_publisher: (i64::MAX, i64::MIN),
        }
    }
}

struct DisplayAsDotEdge {
    source: usize,
    target: usize,
    latencies: Sorted<i64>,
    node_index: Option<usize>,
    edge_type: EdgeType,
}

pub struct DisplayAsDot<'a> {
    graph_node_to_ros_node: HashMap<Node, ArcMutWrapper<model::Node>>,
    node_to_id: HashMap<Node, usize>,
    ros_nodes: Vec<ArcMutWrapper<model::Node>>,
    ros_node_to_id: HashMap<ArcMutWrapper<model::Node>, usize>,
    ros_nodes_min_max_latency_stats: HashMap<ArcMutWrapper<model::Node>, EdgeWeightStats>,

    edges: Vec<DisplayAsDotEdge>,
    pub_sub_latency_range: Option<(i64, i64)>,

    analysis: &'a DependencyGraph,
}

impl<'a> DisplayAsDot<'a> {
    pub fn new(graph: &'a DependencyGraph) -> Self {
        let mut graph_node_to_ros_node: HashMap<Node, ArcMutWrapper<model::Node>> = HashMap::new();
        let mut node_to_id = HashMap::new();

        let mut graph_node_id = 1;

        for publisher in graph.publisher_nodes.keys() {
            let node = Node::Publisher(publisher.clone());
            node_to_id.insert(node.clone(), graph_node_id);
            graph_node_id += 1;

            let publisher = publisher.0.lock().unwrap();
            if let Known::Known(ros_node_arc) = publisher.get_node() {
                let ros_node = ros_node_arc.get_arc().expect("Node should be alive");
                graph_node_to_ros_node.insert(node, ros_node.clone().into());
            }
        }

        for subscriber in graph.subscriber_nodes.keys() {
            let node = Node::Subscriber(subscriber.clone());
            node_to_id.insert(node.clone(), graph_node_id);
            graph_node_id += 1;

            let subscriber = subscriber.0.lock().unwrap();
            if let Known::Known(ros_node_arc) = subscriber.get_node() {
                let ros_node = ros_node_arc.get_arc().expect("Node should be alive");
                graph_node_to_ros_node.insert(node, ros_node.clone().into());
            }
        }

        for timer in graph.timer_nodes.keys() {
            let node = Node::Timer(timer.clone());
            node_to_id.insert(node.clone(), graph_node_id);
            graph_node_id += 1;

            let timer = timer.0.lock().unwrap();
            let ros_node = timer.get_node().unwrap().get_arc().unwrap();
            graph_node_to_ros_node.insert(node, ros_node.clone().into());
        }

        for callback in graph.callback_nodes.keys() {
            let node = Node::Callback(callback.clone());
            node_to_id.insert(node.clone(), graph_node_id);
            graph_node_id += 1;

            let callback = callback.0.lock().unwrap();
            let ros_node = callback.get_node().unwrap().get_arc().unwrap();
            graph_node_to_ros_node.insert(node, ros_node.clone().into());
        }

        let unique_used_ros_nodes = graph_node_to_ros_node
            .values()
            .collect::<HashSet<_>>()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        let ros_node_to_id = unique_used_ros_nodes
            .iter()
            .enumerate()
            .map(|(id, ros_node)| (ros_node.clone(), id))
            .collect::<HashMap<_, _>>();

        let (edges, ros_nodes_min_max_latency_stats, pub_sub_latency_range) = process_edges(
            &graph.edges,
            &graph_node_to_ros_node,
            &ros_node_to_id,
            &node_to_id,
        );

        Self {
            graph_node_to_ros_node,
            ros_nodes: unique_used_ros_nodes,
            ros_node_to_id,
            node_to_id,
            ros_nodes_min_max_latency_stats,
            edges,
            analysis: graph,
            pub_sub_latency_range,
        }
    }
}

fn process_edges(
    graph_edges: &HashMap<Edge, EdgeData>,
    graph_node_to_ros_node: &HashMap<Node, ArcMutWrapper<model::Node>>,
    ros_node_to_id: &HashMap<ArcMutWrapper<model::Node>, usize>,
    node_to_id: &HashMap<Node, usize>,
) -> (
    Vec<DisplayAsDotEdge>,
    HashMap<ArcMutWrapper<model::Node>, EdgeWeightStats>,
    Option<(i64, i64)>,
) {
    let (mut pub_sub_min_latency, mut pub_sub_max_latency) = (i64::MAX, i64::MIN);
    let mut edges = Vec::new();
    let mut ros_nodes_min_max_latency_stats: HashMap<ArcMutWrapper<model::Node>, EdgeWeightStats> =
        HashMap::new();

    for (edge, edge_data) in graph_edges {
        let latencies = Sorted::from_unsorted(&edge_data.latencies);
        let median = *latencies.median().unwrap();
        let edge_type = edge.as_type();

        let source = edge.source();
        let target = edge.target();
        let source_ros_node = &graph_node_to_ros_node[&source];
        let target_ros_node = &graph_node_to_ros_node[&target];

        let node_id = match edge_type {
            EdgeType::PublisherSubscriberCommunication => {
                pub_sub_max_latency = pub_sub_max_latency.max(median);
                pub_sub_min_latency = pub_sub_min_latency.min(median);
                None
            }
            EdgeType::SubscriberCallbackInvocation if source_ros_node == target_ros_node => {
                ros_nodes_min_max_latency_stats
                    .entry(source_ros_node.clone())
                    .or_default()
                    .update_subscriber_to_callback(median);
                Some(ros_node_to_id[source_ros_node])
            }
            EdgeType::TimerCallbackInvocation if source_ros_node == target_ros_node => {
                ros_nodes_min_max_latency_stats
                    .entry(source_ros_node.clone())
                    .or_default()
                    .update_timer_to_callback(median);
                Some(ros_node_to_id[source_ros_node])
            }
            EdgeType::PublicationInCallback if source_ros_node == target_ros_node => {
                ros_nodes_min_max_latency_stats
                    .entry(source_ros_node.clone())
                    .or_default()
                    .update_callback_to_publisher(median);
                Some(ros_node_to_id[source_ros_node])
            }
            _ => {
                // ServiceToCallback: Service is represented by its callback so the latency is always 0.
                None
            }
        };

        let source_id = node_to_id[&source];
        let target_id = node_to_id[&target];

        edges.push(DisplayAsDotEdge {
            source: source_id,
            target: target_id,
            latencies,
            node_index: node_id,
            edge_type,
        });
    }
    let pub_sub_latency_range =
        if pub_sub_min_latency == i64::MAX && pub_sub_max_latency == i64::MIN {
            None
        } else {
            Some((pub_sub_min_latency, pub_sub_max_latency))
        };

    (
        edges,
        ros_nodes_min_max_latency_stats,
        pub_sub_latency_range,
    )
}

fn get_node_name_and_tooltip(
    node: &Node,
    analysis: &DependencyGraph,
    ros_node_name: Known<&str>,
) -> (String, String) {
    match node {
        Node::Publisher(publisher_arc) => {
            let publisher = publisher_arc.0.lock().unwrap();
            let topic = publisher.get_topic().to_string();
            let name = format!("Publisher\n{topic}");
            let tooltip = format!(
                "Node: {ros_node_name}\nDelay between publications:\n{}",
                DisplayDurationStats::with_newline(
                    &analysis.publisher_nodes[publisher_arc].publication_delay
                )
            );
            (name, tooltip)
        }
        Node::Subscriber(subscriber_arc) => {
            let subscriber = subscriber_arc.0.lock().unwrap();
            let topic = subscriber.get_topic().to_string();
            let name = format!("Subscriber\n{topic}");
            let tooltip = format!(
                "Node: {ros_node_name}\nDelay between messages:\n{}",
                DisplayDurationStats::with_newline(
                    &analysis.subscriber_nodes[subscriber_arc].take_delay
                )
            );
            (name, tooltip)
        }
        Node::Timer(timer_arc) => {
            let timer = timer_arc.0.lock().unwrap();
            let period = timer.get_period().unwrap();
            let name = format!("Timer\n{}", DisplayDuration(period));
            let tooltip = format!(
                "Node: {ros_node_name}\nDelay between activations:\n{}",
                DisplayDurationStats::with_newline(
                    &analysis.timer_nodes[timer_arc].activation_delay
                )
            );
            (name, tooltip)
        }
        Node::Callback(callback_arc) => {
            let callback = callback_arc.0.lock().unwrap();
            let name = format!(
                "Callback\n{}",
                Known::<&CallbackCaller>::from(callback.get_caller())
            );
            let tooltip = format!(
                "Node: {ros_node_name}\nDelay between activations:\n{}\nExecution duration:\n{}",
                DisplayDurationStats::with_newline(
                    &analysis.callback_nodes[callback_arc].activation_delay
                ),
                DisplayDurationStats::with_newline(
                    &analysis.callback_nodes[callback_arc].durations
                )
            );
            (name, tooltip)
        }
        Node::Service(service_arc) => {
            let service = service_arc.0.lock().unwrap();
            let name = format!("Service\n{}", service.get_name());
            let tooltip = format!("Node: {ros_node_name}\nSee callback for details",);
            (name, tooltip)
        }
    }
}

impl<'a> std::fmt::Display for DisplayAsDot<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cluster_names = self
            .ros_nodes
            .iter()
            .map(|node| node.0.lock().unwrap().get_full_name().to_string())
            .collect::<Vec<_>>();

        let mut clusters = vec![Vec::new(); self.ros_node_to_id.len()];
        for (node, ros_node) in &self.graph_node_to_ros_node {
            let id = self.ros_node_to_id[ros_node];
            let graph_node_id = self.node_to_id[node];
            clusters[id].push(graph_node_id);
        }

        let mut graph = graphviz_export::Graph::new();
        graph.set_attribute("rankdir", "LR");
        for (node, id) in &self.node_to_id {
            let ros_node_name =
                self.graph_node_to_ros_node
                    .get(node)
                    .map_or(Known::Unknown, |node_arc| {
                        node_arc
                            .0
                            .lock()
                            .unwrap()
                            .get_full_name()
                            .map(ToString::to_string)
                    });
            let (node_name, tooltip) =
                get_node_name_and_tooltip(node, self.analysis, ros_node_name.as_deref());

            let graph_node = graph.add_node(&node_name, *id);
            graph_node.set_shape(NodeShape::Ellipse);
            graph_node.set_attribute("tooltip", &tooltip);
        }

        for edge in &self.edges {
            let graph_edge = graph.add_edge(edge.source, edge.target, "");
            graph_edge.set_attribute(
                "tooltip",
                &format!(
                    "Latency:\n{}",
                    DisplayDurationStats::with_newline(&edge.latencies),
                ),
            );

            if let Some((min_latency, max_latency)) = match edge.edge_type {
                EdgeType::PublisherSubscriberCommunication => self.pub_sub_latency_range,
                _ => {
                    if let Some(node_id) = edge.node_index {
                        self.ros_nodes_min_max_latency_stats[&self.ros_nodes[node_id]]
                            .range_for_type(edge.edge_type)
                    } else {
                        None
                    }
                }
            } {
                graph_edge.set_attribute(
                    "color",
                    &COLOR_GRADIENT
                        .color_for_range_with_min_multiplier(
                            *edge.latencies.median().unwrap(),
                            min_latency,
                            max_latency,
                        )
                        .to_string(),
                );
            }
        }

        for (cluster_nodes, cluster_name) in clusters.into_iter().zip(cluster_names) {
            graph.add_cluster(&cluster_name, cluster_nodes);
        }

        write!(f, "{graph}")
    }
}

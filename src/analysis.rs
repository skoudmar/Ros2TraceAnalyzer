use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use derive_more::derive::From;
use graph::Graph;

use crate::events_common::Context;
use crate::model::{
    Callback, CallbackInstance, CallbackTrigger, CallbackType, PublicationMessage, Publisher,
    Subscriber, SubscriptionMessage,
};
use crate::processed_events::{ros2, Event, FullEvent};
use crate::utils::{DurationDisplayImprecise, Known};

pub trait EventAnalysis {
    /// Initialize the analysis
    ///
    /// This method is called before any events are processed
    fn initialize(&mut self);

    /// Process an event
    fn process_event(&mut self, event: &FullEvent);

    /// Finalize the analysis
    ///
    /// This method is called after all events have been processed
    fn finalize(&mut self);
}

#[derive(Debug, From)]
struct ArcMutWrapper<T>(Arc<Mutex<T>>);

impl<T> PartialEq for ArcMutWrapper<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Clone for ArcMutWrapper<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Eq for ArcMutWrapper<T> {}

impl<T> Hash for ArcMutWrapper<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

type SubPubKey = (ArcMutWrapper<Subscriber>, Option<ArcMutWrapper<Publisher>>);
pub struct MessageLatency {
    messages: HashSet<ArcMutWrapper<SubscriptionMessage>>,
    latencies: HashMap<SubPubKey, Vec<i64>>,
}

#[derive(Debug)]
pub struct MessageLatencyStats {
    topic: String,
    subscriber: Arc<Mutex<Subscriber>>,
    publisher: Option<Arc<Mutex<Publisher>>>,
    message_count: usize,
    max_latency: i64,
    min_latency: i64,
    avg_latency: i64,
}

impl PartialEq for MessageLatencyStats {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl PartialOrd for MessageLatencyStats {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.topic.partial_cmp(&other.topic).and_then(|ord| {
            if ord != Ordering::Equal {
                return Some(ord);
            }

            if Arc::ptr_eq(&self.subscriber, &other.subscriber) {
                return Some(Ordering::Equal);
            }
            let sub1 = self.subscriber.lock().unwrap();
            let sub2 = other.subscriber.lock().unwrap();

            let node1 = sub1.get_node();
            let node2 = sub2.get_node();

            let (node1, node2) = match (node1, node2) {
                (Known::Known(_), Known::Unknown) => return Some(Ordering::Greater),
                (Known::Unknown, Known::Known(_)) => return Some(Ordering::Less),
                (Known::Unknown, Known::Unknown) => return None,
                (Known::Known(node1), Known::Known(node2)) => (node1.upgrade(), node2.upgrade()),
            };

            let (node1, node2) = match (node1, node2) {
                (Some(_), None) => return Some(Ordering::Greater),
                (None, Some(_)) => return Some(Ordering::Less),
                (None, None) => return None,
                (Some(node1), Some(node2)) => (node1, node2),
            };

            if Arc::ptr_eq(&node1, &node2) {
                return Some(Ordering::Equal);
            }

            let node1 = node1.lock().unwrap();
            let node2 = node2.lock().unwrap();

            match (node1.get_name(), node2.get_name()) {
                (Known::Known(_), Known::Unknown) => return Some(Ordering::Greater),
                (Known::Unknown, Known::Known(_)) => return Some(Ordering::Less),
                (Known::Unknown, Known::Unknown) => return None,
                (Known::Known(name1), Known::Known(name2)) => Some(name1.cmp(name2)),
            }
        })
    }
}

impl MessageLatency {
    pub fn new() -> Self {
        Self {
            messages: HashSet::new(),
            latencies: HashMap::new(),
        }
    }

    fn add_message(&mut self, message: Arc<Mutex<SubscriptionMessage>>) {
        self.messages.insert(message.into());
    }

    fn calculate_latency_and_get_publisher(
        message: &SubscriptionMessage,
    ) -> (Option<i64>, Option<ArcMutWrapper<Publisher>>) {
        let receive_time = message
            .get_receive_time()
            .expect("Receive time should be known");
        let (send_time, publisher) =
            if let Some(publication_message) = message.get_publication_message() {
                let publication_message = publication_message.lock().unwrap();
                let send_time = publication_message
                    .get_publication_time()
                    .expect("Publication time should be known");
                let publisher = publication_message.get_publisher().map(Into::into);

                (Some(send_time), publisher)
            } else if let Some(publication_timestamp) = message.get_sender_timestamp() {
                // If the publication message is not available, use the sender timestamp
                (Some(publication_timestamp), None)
            } else {
                eprintln!("No publication message or timestamp found for message {message:?}");
                (None, None)
            };

        send_time.inspect(|send_time| {
            assert!(*send_time <= receive_time);
        });

        let latency =
            send_time.map(|send_time| receive_time.timestamp_nanos() - send_time.timestamp_nanos());

        (latency, publisher)
    }

    fn remove_message(&mut self, message: Arc<Mutex<SubscriptionMessage>>) {
        let message = message.into();
        if self.messages.remove(&message) {
            let message = message.0.lock().unwrap();
            let (latency_ns, publisher) = Self::calculate_latency_and_get_publisher(&message);

            if message.get_subscriber().is_none() {
                // The message is missing the subscriber. The latency series cannot be identified.
                return;
            }

            self.latencies
                .entry((message.get_subscriber().unwrap().into(), publisher))
                .or_default()
                .push(latency_ns.unwrap());
        }
    }

    fn remove_remaining_messages(&mut self) {
        for message in self.messages.drain() {
            let message = message.0.lock().unwrap();
            let (latency_ns, publisher) = Self::calculate_latency_and_get_publisher(&message);

            if message.get_subscriber().is_none() {
                // The message is missing the subscriber. The latency series cannot be identified.
                continue;
            }

            self.latencies
                .entry((message.get_subscriber().unwrap().into(), publisher))
                .or_default()
                .push(latency_ns.unwrap());
        }
    }

    pub fn calculate_stats(&self) -> Vec<MessageLatencyStats> {
        self.latencies
            .iter()
            .map(|((subscriber_arc, publisher_arc), latencies)| {
                let subscriber = subscriber_arc.0.lock().unwrap();
                let topic = subscriber.get_topic();
                let lat_len = latencies.len();
                let min_latency = *latencies.iter().min().unwrap();
                let max_latency = *latencies.iter().max().unwrap();
                let avg_latency = (latencies.iter().copied().map(i128::from).sum::<i128>()
                    / lat_len as i128) as i64;

                MessageLatencyStats {
                    topic: topic.to_string(),
                    subscriber: subscriber_arc.0.clone(),
                    publisher: publisher_arc.as_ref().map(|p| p.0.clone()),
                    message_count: lat_len,
                    max_latency,
                    min_latency,
                    avg_latency,
                }
            })
            .collect()
    }

    pub(crate) fn print_stats(&self) {
        println!("Message latency statistics:");
        let mut stats = self.calculate_stats();
        stats.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        for (i, stat) in stats.iter().enumerate() {
            let subscriber = stat.subscriber.lock().unwrap();
            let topic = &stat.topic;
            let publisher = stat.publisher.as_ref().map(|p| p.lock().unwrap());
            let msg_count = stat.message_count;

            println!("- [{i:4}] Topic {topic}:");
            println!("    Subscriber: {subscriber:#}");
            if let Some(publisher) = publisher {
                println!("    Publisher: {publisher}");
            } else {
                println!("    Publisher: Unknown");
            }
            println!("    Message count: {msg_count}");
            if msg_count > 0 {
                println!(
                    "    Max latency: {}",
                    DurationDisplayImprecise(stat.max_latency)
                );
                println!(
                    "    Min latency: {}",
                    DurationDisplayImprecise(stat.min_latency)
                );
                println!(
                    "    Avg latency: {}",
                    DurationDisplayImprecise(stat.avg_latency)
                );
            }
        }
    }
}

impl EventAnalysis for MessageLatency {
    fn initialize(&mut self) {
        self.messages.clear();
        self.latencies.clear();
    }

    fn process_event(&mut self, event: &FullEvent) {
        match &event.event {
            Event::Ros2(ros2::Event::RmwTake(event)) => {
                self.add_message(event.message.clone());
            }
            Event::Ros2(ros2::Event::RclTake(event)) => {
                let message = event.message.clone();
                if event.is_new {
                    self.add_message(message);
                } else {
                    assert!(self.messages.contains(&message.into()));
                }
            }
            Event::Ros2(ros2::Event::RclCppTake(event)) => {
                let message = event.message.clone();
                if event.is_new {
                    self.add_message(message);
                } else {
                    assert!(self.messages.contains(&message.into()));
                }
            }

            _ => {}
        }
    }

    fn finalize(&mut self) {
        // Make sure all messages are accounted for. The remaining messages are
        // missing the RclCppTake event.
        self.remove_remaining_messages();
    }
}

#[derive(Debug)]
pub struct CallbackDuration {
    durations: HashMap<ArcMutWrapper<Callback>, Vec<i64>>,
    started_callbacks: HashSet<ArcMutWrapper<CallbackInstance>>,
    not_ended_callbacks: Vec<ArcMutWrapper<CallbackInstance>>,
}

impl CallbackDuration {
    pub fn new() -> Self {
        Self {
            durations: HashMap::new(),
            started_callbacks: HashSet::new(),
            not_ended_callbacks: Vec::new(),
        }
    }

    fn calculate_duration(callback: &CallbackInstance) -> Option<i64> {
        let start_time = callback.get_start_time();
        let end_time = callback.get_end_time()?;

        assert!(end_time >= start_time);
        let duration = end_time.timestamp_nanos() - start_time.timestamp_nanos();

        Some(duration)
    }

    fn start_callback(&mut self, callback: Arc<Mutex<CallbackInstance>>) {
        self.started_callbacks.insert(callback.into());
    }

    fn end_callback(&mut self, callback: Arc<Mutex<CallbackInstance>>) {
        let callback = callback.into();
        if self.started_callbacks.remove(&callback) {
            let callback_instance = callback.0.lock().unwrap();
            let duration = Self::calculate_duration(&callback_instance)
                .expect("Duration should be known in callback_end");

            self.durations
                .entry(callback_instance.get_callback().into())
                .or_default()
                .push(duration);
        } else {
            panic!("Callback {callback:?} was not started");
        }
    }

    fn end_remaining_callbacks(&mut self) {
        assert!(self.not_ended_callbacks.is_empty());
        self.not_ended_callbacks = self
            .started_callbacks
            .drain()
            .map(|callback| {
                {
                    let callback_instance = callback.0.lock().unwrap();
                    if let Some(duration) = Self::calculate_duration(&callback_instance) {
                        unreachable!(
                            "Callback {callback:?} was not ended but has duration {duration}"
                        );
                    };
                }

                callback
            })
            .collect();
    }

    pub(crate) fn print_stats(&self) {
        println!("Callback duration statistics:");
        for (i, (callback, durations)) in self.durations.iter().enumerate() {
            let callback = callback.0.lock().unwrap();
            let dur_len = durations.len();
            let min_duration = durations.iter().min().copied().unwrap();
            let max_duration = durations.iter().max().copied().unwrap();
            let avg_duration = durations.iter().sum::<i64>() / dur_len as i64;

            println!("- [{i:4}] Callback {callback}:");
            println!("    Call count: {dur_len}");
            if dur_len > 0 {
                println!(
                    "    Max duration: {}",
                    DurationDisplayImprecise(max_duration)
                );
                println!(
                    "    Min duration: {}",
                    DurationDisplayImprecise(min_duration)
                );
                println!(
                    "    Avg duration: {}",
                    DurationDisplayImprecise(avg_duration)
                );
            }
        }
    }
}

impl EventAnalysis for CallbackDuration {
    fn initialize(&mut self) {
        self.durations.clear();
        self.started_callbacks.clear();
        self.not_ended_callbacks.clear();
    }

    fn process_event(&mut self, event: &FullEvent) {
        match &event.event {
            Event::Ros2(ros2::Event::CallbackStart(event)) => {
                self.start_callback(event.callback.clone());
            }
            Event::Ros2(ros2::Event::CallbackEnd(event)) => {
                self.end_callback(event.callback.clone());
            }

            _ => {}
        }
    }

    fn finalize(&mut self) {
        // Make sure all started callbacks are ended. The remaining callbacks are
        // missing the CallbackEnd event.
        self.end_remaining_callbacks();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Id {
    vtid: u32,
    hostname: String,
}

#[derive(Debug, Default)]
pub struct PublicationInCallback {
    active_callbacks: HashMap<Id, ArcMutWrapper<CallbackInstance>>,
    dependency: HashSet<(ArcMutWrapper<Publisher>, ArcMutWrapper<Callback>)>,
}

impl PublicationInCallback {
    pub fn new() -> Self {
        Self::default()
    }

    fn activate_callback(&mut self, callback: Arc<Mutex<CallbackInstance>>, context: &Context) {
        let id = Id {
            vtid: context.vtid(),
            hostname: context.hostname().to_string(),
        };

        if let Some(old) = self.active_callbacks.insert(id, callback.into()) {
            panic!(
                "Callback {old:?} is already active on vtid {} on host {}",
                context.vtid(),
                context.hostname()
            );
        }
    }

    fn deactivate_callback(&mut self, callback: Arc<Mutex<CallbackInstance>>, context: &Context) {
        let id = Id {
            vtid: context.vtid(),
            hostname: context.hostname().to_string(),
        };

        if let Some(old) = self.active_callbacks.remove(&id) {
            if !Arc::ptr_eq(&old.0, &callback) {
                panic!(
                    "Callback {old:?} is not active on vtid {} on host {}",
                    context.vtid(),
                    context.hostname(),
                );
            }
        } else {
            panic!(
                "No callback is being executed on vtid {} on host {}",
                context.vtid(),
                context.hostname(),
            );
        }
    }

    fn process_publication(
        &mut self,
        publication: Arc<Mutex<PublicationMessage>>,
        context: &Context,
    ) {
        let id = Id {
            vtid: context.vtid(),
            hostname: context.hostname().to_string(),
        };

        if let Some(callback) = self.active_callbacks.get(&id) {
            let callback = callback.0.lock().unwrap();
            let publication_msg = publication.lock().unwrap();
            let publisher_arc = publication_msg.get_publisher().unwrap();

            self.dependency
                .insert((publisher_arc.into(), callback.get_callback().into()));
        }
    }

    fn get_dependency(&self) -> &HashSet<(ArcMutWrapper<Publisher>, ArcMutWrapper<Callback>)> {
        &self.dependency
    }
}

impl EventAnalysis for PublicationInCallback {
    fn initialize(&mut self) {
        *self = Self::default();
    }

    fn process_event(&mut self, full_event: &FullEvent) {
        match &full_event.event {
            Event::Ros2(ros2::Event::CallbackStart(event)) => {
                self.activate_callback(event.callback.clone(), &full_event.context);
            }
            Event::Ros2(ros2::Event::CallbackEnd(event)) => {
                self.deactivate_callback(event.callback.clone(), &full_event.context);
            }
            Event::Ros2(ros2::Event::RmwPublish(event)) => {
                self.process_publication(event.message.clone(), &full_event.context);
            }
            _ => {}
        }
    }

    fn finalize(&mut self) {
        // Nothing to do
    }
}

pub mod graph {
    use crate::model::Callback;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, Default)]
    pub struct Graph {
        nodes: Vec<Node>,
        edges: Vec<(usize, usize)>,

        sources: Vec<usize>,
    }

    impl Graph {
        pub(super) fn new(
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
                println!("    [{i:4}] {callback}");
            }

            println!("  Edges:");
            for (src, dst) in self.edges().iter() {
                println!("    {src} -> {dst}");
            }

            println!("Sources: {:?}", self.sources());
        }
    }

    #[derive(Debug, Clone)]
    pub struct Node {
        callback: Arc<Mutex<Callback>>,
    }

    impl Node {
        pub(super) fn new(callback: Arc<Mutex<Callback>>) -> Self {
            Self { callback }
        }

        pub fn callback(&self) -> &Arc<Mutex<Callback>> {
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

    fn add_callback(&mut self, callback: Arc<Mutex<Callback>>) {
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

            let subscriber = subscriber.upgrade().expect("Subscriber should be alive");
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
}

impl EventAnalysis for CallbackDependency {
    fn initialize(&mut self) {
        // Clear all data
        *self = Self::default();
    }

    fn process_event(&mut self, full_event: &FullEvent) {
        self.publication_in_callback.process_event(full_event);
        if let Event::Ros2(ros2::Event::RclcppCallbackRegister(event)) = &full_event.event {
            self.add_callback(event.callback.clone());
        }
    }

    fn finalize(&mut self) {
        self.construct_callback_graph();
    }
}

#[derive(Debug, Default)]
pub struct MessageTakeToCallbackLatency {
    latencies: HashMap<ArcMutWrapper<Callback>, Vec<i64>>,
}

impl MessageTakeToCallbackLatency {
    pub fn new() -> Self {
        Self::default()
    }

    fn process_callback_start(&mut self, callback: Arc<Mutex<CallbackInstance>>) {
        let callback_instance = callback.lock().unwrap();

        match callback_instance.get_trigger() {
            CallbackTrigger::SubscriptionMessage(msg) => {
                let message = msg.lock().unwrap();
                let receive_time = message.get_receive_time().unwrap();
                let start_time = callback_instance.get_start_time();
                let latency = start_time.timestamp_nanos() - receive_time.timestamp_nanos();

                self.latencies
                    .entry(callback_instance.get_callback().into())
                    .or_default()
                    .push(latency);
            }
            _ => {
                // Ignore other triggers
            }
        }
    }

    pub(crate) fn print_stats(&self) {
        println!("Message take to callback latency statistics:");
        for (i, (callback, latencies)) in self.latencies.iter().enumerate() {
            let callback = callback.0.lock().unwrap();
            let dur_len = latencies.len();
            let min_latency = *latencies.iter().min().unwrap();
            let max_latency = *latencies.iter().max().unwrap();
            let avg_latency = latencies.iter().sum::<i64>() / dur_len as i64;

            println!("- [{i:4}] Callback {callback}:");
            println!("    Call count: {dur_len}");
            if dur_len > 0 {
                println!("    Max latency: {}", DurationDisplayImprecise(max_latency));
                println!("    Min latency: {}", DurationDisplayImprecise(min_latency));
                println!("    Avg latency: {}", DurationDisplayImprecise(avg_latency));
            }
        }
    }
}

impl EventAnalysis for MessageTakeToCallbackLatency {
    fn initialize(&mut self) {
        self.latencies.clear();
    }

    fn process_event(&mut self, full_event: &FullEvent) {
        if let Event::Ros2(ros2::Event::CallbackStart(event)) = &full_event.event {
            self.process_callback_start(event.callback.clone());
        }
    }

    fn finalize(&mut self) {
        // Nothing to do
    }
}

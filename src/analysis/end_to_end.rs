use core::time;
use std::collections::HashMap;
use std::ops::Not;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::events_common::Context;
use crate::model::display::get_node_name_from_weak;
use crate::model::{self, CallbackInstance, PublicationMessage, SubscriptionMessage, Time};
use crate::processed_events::{ros2, Event, FullEvent};
use crate::utils::Known;

use super::{AnalysisOutput, ArcMutWrapper, EventAnalysis};

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Thing {
    Publisher(ArcMutWrapper<model::Publisher>),
    Subscriber(ArcMutWrapper<model::Subscriber>),
    Callback(ArcMutWrapper<model::Callback>),
}

impl Thing {
    fn get_as_string(&self) -> String {
        match self {
            Thing::Publisher(publisher) => {
                format!("pub: {}", publisher.0.lock().unwrap().get_topic())
            }
            Thing::Subscriber(subscriber) => {
                format!("sub: {}", subscriber.0.lock().unwrap().get_topic())
            }
            Thing::Callback(callback) => format!(
                "cb: {}",
                callback
                    .0
                    .lock()
                    .unwrap()
                    .get_caller()
                    .unwrap()
                    .get_caller_as_string()
            ),
        }
    }

    fn get_node(&self) -> String {
        match self {
            Thing::Publisher(publisher) => {
                let node = publisher.0.lock().unwrap().get_node().unwrap();
                get_node_name_from_weak(&node.get_weak()).unwrap()
            }
            Thing::Subscriber(subscriber) => {
                let node = subscriber.0.lock().unwrap().get_node().unwrap();
                get_node_name_from_weak(&node.get_weak()).unwrap()
            }
            Thing::Callback(callback) => {
                let node = callback.0.lock().unwrap().get_node().unwrap();
                get_node_name_from_weak(&node.get_weak()).unwrap()
            }
        }
    }
}

#[derive(Debug)]
enum ChainPart {
    MessagePublication {
        pub_msg: ArcMutWrapper<PublicationMessage>,
        previous: Option<Arc<ChainPart>>,
        extended: AtomicBool,
    },
    MessageReception {
        sub_msg: ArcMutWrapper<SubscriptionMessage>,
        previous: Option<Arc<ChainPart>>,
        extended: AtomicBool,
    },
    CallbackInvocation {
        callback: ArcMutWrapper<CallbackInstance>,
        previous: Option<Arc<ChainPart>>,
        extended: AtomicBool,
    },
}

impl ChainPart {
    fn get_previous(&self) -> Option<&Arc<ChainPart>> {
        match self {
            ChainPart::MessageReception { previous, .. }
            | ChainPart::MessagePublication { previous, .. }
            | ChainPart::CallbackInvocation { previous, .. } => previous.as_ref(),
        }
    }

    fn set_extended(&self) {
        match self {
            ChainPart::MessageReception { extended, .. }
            | ChainPart::MessagePublication { extended, .. }
            | ChainPart::CallbackInvocation { extended, .. } => {
                extended.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }

    fn is_extended(&self) -> bool {
        match self {
            ChainPart::MessageReception { extended, .. }
            | ChainPart::MessagePublication { extended, .. }
            | ChainPart::CallbackInvocation { extended, .. } => {
                extended.load(std::sync::atomic::Ordering::SeqCst)
            }
        }
    }

    fn get_time_and_thing(&self) -> (Time, Thing) {
        match self {
            ChainPart::MessagePublication { pub_msg, .. } => {
                let pub_msg = pub_msg.0.lock().unwrap();
                let publisher = pub_msg.get_publisher().unwrap();

                (
                    pub_msg.get_publication_time().unwrap(),
                    Thing::Publisher(publisher.into()),
                )
            }
            ChainPart::MessageReception { sub_msg, .. } => {
                let sub_msg = sub_msg.0.lock().unwrap();
                let subscriber = sub_msg.get_subscriber().unwrap();

                let time = sub_msg.get_receive_time().unwrap();
                (time, Thing::Subscriber(subscriber.into()))
            }
            ChainPart::CallbackInvocation { callback, .. } => {
                let callback_inst = callback.0.lock().unwrap();
                let callback = callback_inst.get_callback();
                let time = callback_inst.get_start_time();

                (time, Thing::Callback(callback.into()))
            }
        }
    }
}

#[derive(Default)]
pub struct EndToEndAnalysis {
    message_publication: HashMap<ArcMutWrapper<PublicationMessage>, Arc<ChainPart>>,
    message_reception: HashMap<ArcMutWrapper<SubscriptionMessage>, Arc<ChainPart>>,
    callback_invocation: HashMap<ArcMutWrapper<CallbackInstance>, Arc<ChainPart>>,

    // last_spin_wake_up_time_for_node: HashMap<ArcMutWrapper<model::Node>, Time>,
    running_callbacks: HashMap<ThreadId, Arc<Mutex<CallbackInstance>>>,

    reductions: HashMap<(Thing, Thing), Vec<i64>>,
}

impl EndToEndAnalysis {
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
        let previous = match callback_instance.get_trigger() {
            model::CallbackTrigger::SubscriptionMessage(msg) => {
                let msg_arc = msg.clone().into();
                self.message_reception
                    .get(&msg_arc)
                    .cloned()
                    .inspect(|prev| prev.set_extended())
            }
            // TODO: Handle other callback triggers
            model::CallbackTrigger::Service(_request) => None,
            model::CallbackTrigger::Timer(_timer) => None,
        };

        let chain_part = ChainPart::CallbackInvocation {
            callback: event.callback.clone().into(),
            previous,
            extended: AtomicBool::new(false),
        };

        let chain_part = Arc::new(chain_part);
        self.callback_invocation
            .insert(event.callback.clone().into(), chain_part.clone());
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
    }

    fn process_rmw_take(&mut self, event: &ros2::RmwTake, event_time: Time) {
        if !event.taken {
            // Only process taken messages
            return;
        }
        let message = event.message.lock().unwrap();
        let sub_msg_arc: ArcMutWrapper<_> = event.message.clone().into();

        let previous = if let Some(pub_msg) = message.get_publication_message() {
            let pub_msg = pub_msg.clone().into();
            self.message_publication
                .get(&pub_msg)
                .cloned()
                .inspect(|prev| prev.set_extended())
        } else {
            None
        };

        let chain_part = ChainPart::MessageReception {
            sub_msg: sub_msg_arc.clone(),
            previous,
            extended: AtomicBool::new(false),
        };
        let chain_part = Arc::new(chain_part);
        self.message_reception
            .insert(sub_msg_arc, chain_part.clone());
    }

    fn process_publication(
        &mut self,
        event: &ros2::RmwPublish,
        event_time: Time,
        context: &Context,
    ) {
        let pub_msg_arc: ArcMutWrapper<_> = event.message.clone().into();
        let message = event.message.lock().unwrap();

        let previous =
            if let Some(callback_instance_arc) = self.running_callbacks.get(&context.into()) {
                let callback_instance_arc = callback_instance_arc.clone().into();

                self.callback_invocation
                    .get(&callback_instance_arc)
                    .cloned()
                    .inspect(|prev| prev.set_extended())
            } else {
                None
            };

        let chain_part = ChainPart::MessagePublication {
            pub_msg: pub_msg_arc.clone(),
            previous,
            extended: AtomicBool::new(false),
        };
        let chain_part = Arc::new(chain_part);

        self.message_publication
            .insert(pub_msg_arc, chain_part.clone());
    }

    fn reduce_chain_parts(&mut self) {
        for chain_part in self
            .message_publication
            .values()
            .chain(self.message_reception.values())
            .chain(self.callback_invocation.values())
        {
            if chain_part.is_extended() {
                // Not the end of the chain - skip
                continue;
            }

            let mut beginning = chain_part.clone();
            while let Some(previous) = beginning.get_previous() {
                if let ChainPart::MessagePublication { pub_msg, .. } = previous.as_ref() {
                    let publisher = pub_msg.0.lock().unwrap().get_publisher().unwrap();
                    let publisher = publisher.lock().unwrap();
                    let topic = publisher.get_topic();

                    if topic == Known::from("/carla/ego_vehicle/imu") {
                        // Skip the IMU topic
                        break;
                    }
                }
                beginning = previous.clone();
            }

            let (chain_start_time, start_thing) = beginning.get_time_and_thing();
            let (chain_end_time, end_thing) = match chain_part.as_ref() {
                ChainPart::MessagePublication { pub_msg, .. } => {
                    let pub_msg = pub_msg.0.lock().unwrap();
                    let publisher = pub_msg.get_publisher().unwrap();

                    (
                        pub_msg.get_publication_time().unwrap(),
                        Thing::Publisher(publisher.into()),
                    )
                }
                ChainPart::MessageReception { sub_msg, .. } => {
                    let sub_msg = sub_msg.0.lock().unwrap();
                    let subscriber = sub_msg.get_subscriber().unwrap();

                    let time = sub_msg.get_receive_time().unwrap();
                    (time, Thing::Subscriber(subscriber.into()))
                }
                ChainPart::CallbackInvocation { callback, .. } => {
                    let callback_inst = callback.0.lock().unwrap();
                    let callback = callback_inst.get_callback();
                    let time = callback_inst
                        .get_end_time()
                        .unwrap_or_else(|| callback_inst.get_start_time());

                    (time, Thing::Callback(callback.into()))
                }
            };

            if start_thing == end_thing {
                // Skip if the start and end are the same
                continue;
            }

            let chain_duration =
                chain_end_time.timestamp_nanos() - chain_start_time.timestamp_nanos();

            self.reductions
                .entry((start_thing, end_thing))
                .or_default()
                .push(chain_duration);
        }
    }

    pub fn print_reductions(&self) {
        for ((start_thing, end_thing), durations) in &self.reductions {
            println!(
                "E2E latency from {} to {}: {:?}",
                start_thing.get_as_string(),
                end_thing.get_as_string(),
                durations
            );
        }
    }
}

impl EventAnalysis for EndToEndAnalysis {
    fn initialize(&mut self) {
        *self = Self::default();
    }

    fn process_event(&mut self, full_event: &FullEvent) {
        let event_time = full_event.time;

        match &full_event.event {
            Event::Ros2(ros2::Event::CallbackStart(event)) => {
                self.process_callback_start(event, event_time, &full_event.context);
            }
            Event::Ros2(ros2::Event::CallbackEnd(event)) => {
                self.process_callback_end(event, event_time, &full_event.context);
            }

            Event::Ros2(ros2::Event::RmwTake(event)) => {
                self.process_rmw_take(event, event_time);
            }

            Event::Ros2(ros2::Event::RmwPublish(event)) => {
                self.process_publication(event, event_time, &full_event.context);
            }

            // Event::R2r(r2r::Event::SpinWake(event)) => {
            //     self.last_spin_wake_up_time_for_node
            //         .insert(event.node.clone().into(), event_time);
            // }
            // Event::R2r(r2r::Event::SpinEnd(event)) => {
            //     debug_assert!(
            //         self.last_spin_wake_up_time_for_node
            //             .contains_key(&event.node.clone().into()),
            //         "Missing spin wake up event."
            //     );
            // }
            _ => {}
        }
    }

    fn finalize(&mut self) {
        self.reduce_chain_parts();
    }
}

#[derive(serde::Serialize, PartialEq, Eq)]
struct JsonEntry {
    beginning_node: String,
    beginning: String,
    end_node: String,
    end: String,
    durations: Vec<i64>,
}

impl PartialOrd for JsonEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JsonEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.beginning_node
            .cmp(&other.beginning_node)
            .then_with(|| self.beginning.cmp(&other.beginning))
            .then_with(|| self.end_node.cmp(&other.end_node))
            .then_with(|| self.end.cmp(&other.end))
            .then_with(|| self.durations.len().cmp(&other.durations.len()))
            .then_with(|| self.durations.cmp(&other.durations))
    }
}

impl AnalysisOutput for EndToEndAnalysis {
    fn write_json(&self, file: &mut std::io::BufWriter<std::fs::File>) -> serde_json::Result<()> {
        let mut json_entries = Vec::new();
        for ((start_thing, end_thing), durations) in &self.reductions {
            let json_entry = JsonEntry {
                beginning_node: start_thing.get_node(),
                beginning: start_thing.get_as_string(),
                end_node: end_thing.get_node(),
                end: end_thing.get_as_string(),
                durations: durations.clone(),
            };
            json_entries.push(json_entry);
        }

        json_entries.sort_unstable();

        serde_json::to_writer(file, &json_entries)
    }
}

struct LatencyForest {
    roots: HashMap<Thing, TreeNode>,
}

impl LatencyForest {
    fn new() -> Self {
        Self {
            roots: HashMap::new(),
        }
    }

    fn insert_chain(&mut self, chain: &Arc<ChainPart>) {
        if chain.get_previous().is_none() {
            // Single node chain is not supported
            return;
        } else if chain.is_extended() {
            panic!("Pass only full chains to this function");
        }

        let mut stack = Vec::new();
        let mut current = chain.clone();
        while let Some(previous) = current.get_previous() {
            let mut to_push = previous.clone();
            std::mem::swap(&mut current, &mut to_push);
            stack.push(to_push);
        }

        let mut parent_id = usize::MAX;
        let (time, thing) = current.get_time_and_thing();
        let mut node = self.roots.entry(thing).or_insert_with(TreeNode::new);
        parent_id = node.add_time(parent_id, time);

        while let Some(current) = stack.pop() {
            let (time, thing) = current.get_time_and_thing();
            node = node.get_child(thing);
            node.add_time(parent_id, time);
        }
    }
}

struct TreeNode {
    /// (id in parent, time)
    times: Vec<(usize, Time)>,
    children: HashMap<Thing, TreeNode>,
}

impl TreeNode {
    fn new() -> Self {
        Self {
            times: Vec::new(),
            children: HashMap::new(),
        }
    }

    /// Returns the id of this entry
    fn add_time(&mut self, parent_id: usize, time: Time) -> usize {
        let id = self.times.len();
        self.times.push((parent_id, time));
        id
    }

    fn get_child(&mut self, child: Thing) -> &mut Self {
        self.children.entry(child).or_insert_with(TreeNode::new)
    }
}

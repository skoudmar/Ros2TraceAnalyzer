use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use derive_more::derive::From;

use crate::model::{Callback, CallbackInstance, Publisher, Subscriber, SubscriptionMessage};
use crate::processed_events::{ros2, Event, FullEvent};

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
    ) -> (i64, Option<ArcMutWrapper<Publisher>>) {
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

                (send_time, publisher)
            } else if let Some(publication_timestamp) = message.get_sender_timestamp() {
                // If the publication message is not available, use the sender timestamp
                (publication_timestamp, None)
            } else {
                panic!("No publication message or timestamp found for message {message:?}");
            };

        assert!(receive_time >= send_time);
        let latency = receive_time.timestamp_nanos() - send_time.timestamp_nanos();

        (latency, publisher)
    }

    fn remove_message(&mut self, message: Arc<Mutex<SubscriptionMessage>>) {
        let message = message.into();
        if self.messages.remove(&message) {
            let message = message.0.lock().unwrap();
            let (latency_ns, publisher) = Self::calculate_latency_and_get_publisher(&message);

            self.latencies
                .entry((message.get_subscriber().unwrap().into(), publisher))
                .or_default()
                .push(latency_ns);
        }
    }

    fn remove_remaining_messages(&mut self) {
        for message in self.messages.drain() {
            let message = message.0.lock().unwrap();
            let (latency_ns, publisher) = Self::calculate_latency_and_get_publisher(&message);

            self.latencies
                .entry((message.get_subscriber().unwrap().into(), publisher))
                .or_default()
                .push(latency_ns);
        }
    }

    pub(crate) fn print_stats(&self) {
        println!("Message latency statistics:");
        for (i, ((subscriber, publisher), latencies)) in self.latencies.iter().enumerate() {
            let subscriber = subscriber.0.lock().unwrap();
            let topic = subscriber.get_topic();
            let publisher = publisher.as_ref().map(|p| p.0.lock().unwrap());
            let lat_len = latencies.len();
            let min_latency = latencies.iter().min().unwrap();
            let max_latency = latencies.iter().max().unwrap();
            let avg_latency =
                latencies.iter().copied().map(i128::from).sum::<i128>() / lat_len as i128;

            println!("- [{i:4}] Topic {topic}:");
            println!("    Subscriber: {subscriber:#}");
            if let Some(publisher) = publisher {
                println!("    Publisher: {publisher:#}");
            } else {
                println!("    Publisher: Unknown");
            }
            println!("    Message count: {lat_len}");
            if lat_len > 0 {
                println!("    Max latency: {max_latency}");
                println!("    Min latency: {min_latency}");
                println!("    Avg latency: {avg_latency}");
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
                assert!(self.messages.contains(&message.into()));
            }
            Event::Ros2(ros2::Event::RclCppTake(event)) => {
                self.remove_message(event.message.clone());
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
                println!("    Max duration: {max_duration}");
                println!("    Min duration: {min_duration}");
                println!("    Avg duration: {avg_duration}");
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

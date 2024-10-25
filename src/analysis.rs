use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use derive_more::derive::From;

use crate::model::{Publisher, Subscriber, SubscriptionMessage};

#[derive(Debug, Clone, From)]
struct ArcMutWrapper<T>(Arc<Mutex<T>>);

impl<T> PartialEq for ArcMutWrapper<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for ArcMutWrapper<T> {}

impl<T> Hash for ArcMutWrapper<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

pub struct MessageLatency {
    messages: HashSet<ArcMutWrapper<SubscriptionMessage>>,
    latencies: HashMap<ArcMutWrapper<Subscriber>, Vec<i64>>,
}

impl MessageLatency {
    pub fn new() -> Self {
        Self {
            messages: HashSet::new(),
            latencies: HashMap::new(),
        }
    }

    pub fn add_message(&mut self, message: Arc<Mutex<SubscriptionMessage>>) {
        self.messages.insert(message.into());
    }

    fn calculate_latency(message: &SubscriptionMessage) -> i64 {
        let receive_time = message
            .get_receive_time()
            .expect("Receive time should be known");
        let send_time = if let Some(publication_message) = message.get_publication_message() {
            let publication_message = publication_message.lock().unwrap();
            publication_message
                .get_publication_time()
                .expect("Publication time should be known")
        } else if let Some(publication_timestamp) = message.get_sender_timestamp() {
            publication_timestamp
        } else {
            panic!("No publication message or timestamp found for message {message:?}");
        };

        assert!(receive_time >= send_time);
        receive_time.timestamp_nanos() - send_time.timestamp_nanos()
    }

    pub fn remove_message(&mut self, message: Arc<Mutex<SubscriptionMessage>>) {
        let message = message.into();
        if self.messages.remove(&message) {
            let message = message.0.lock().unwrap();
            let latency_ns = Self::calculate_latency(&message);

            self.latencies
                .entry(message.get_subscriber().unwrap().into())
                .or_default()
                .push(latency_ns);
        }
    }

    pub fn remove_remaining_messages(&mut self) {
        for message in self.messages.drain() {
            let message = message.0.lock().unwrap();
            let latency_ns = Self::calculate_latency(&message);

            self.latencies
                .entry(message.get_subscriber().unwrap().into())
                .or_default()
                .push(latency_ns);
        }
    }
}

pub struct MessageToReceiveCallback {}

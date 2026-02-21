use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::analyses::analysis::utils::DisplayDurationStats;
use crate::model::display::get_node_name_from_weak;
use crate::model::{Publisher, Subscriber, SubscriptionMessage};
use crate::processed_events::{Event, FullEvent, ros2};
use crate::utils::{DurationDisplayImprecise, Known};

use super::{AnalysisOutput, ArcMutWrapper, EventAnalysis};

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
    latencies: Vec<i64>,
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
                (Known::Known(node1), Known::Known(node2)) => (node1.get_arc(), node2.get_arc()),
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
                (Known::Known(_), Known::Unknown) => Some(Ordering::Greater),
                (Known::Unknown, Known::Known(_)) => Some(Ordering::Less),
                (Known::Unknown, Known::Unknown) => None,
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
                log::warn!("No publication message or timestamp found for message {message:?}");
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

                MessageLatencyStats {
                    topic: topic.to_string(),
                    subscriber: subscriber_arc.0.clone(),
                    publisher: publisher_arc.as_ref().map(|p| p.0.clone()),
                    latencies: latencies.clone(),
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

            println!("- [{i:4}] Topic {topic}:");
            println!("    Subscriber: {subscriber:#}");
            if let Some(publisher) = publisher {
                println!("    Publisher: {publisher}");
            } else {
                println!("    Publisher: Unknown");
            }
            let display = DisplayDurationStats::new(&stat.latencies, "\n\t");
            println!("\t{display}");
            let (mean, std_dev) = display.mean_and_std_dev();
            println!(
                "\tMean: {}, Std. dev.: {}",
                DurationDisplayImprecise(mean),
                DurationDisplayImprecise(std_dev as i64)
            );
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
                    self.add_message(message.clone());
                } else {
                    assert!(self.messages.contains(&message.clone().into()));
                }

                self.remove_message(message);
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

impl AnalysisOutput for MessageLatency {
    fn write_json(&self, file: &mut std::io::BufWriter<std::fs::File>) -> serde_json::Result<()> {
        let stats = self.calculate_stats();
        let stats: Vec<MessageLatencyExport> = stats.into_iter().map(Into::into).collect();
        serde_json::to_writer(file, &stats)
    }

    fn get_binary_output(&self) -> impl Serialize {
        self.calculate_stats()
            .into_iter()
            .map(Into::<MessageLatencyExport>::into)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Serialize)]
struct MessageLatencyExport {
    topic: String,
    subscriber: String,
    publisher: String,
    latencies: Vec<i64>,
}

impl From<MessageLatencyStats> for MessageLatencyExport {
    fn from(value: MessageLatencyStats) -> Self {
        let subscriber = value
            .subscriber
            .lock()
            .map(|s| format!("Subscriber({})", s.get_topic()))
            .unwrap_or("Unknown".into());

        let publisher = value
            .publisher
            .map(|p| {
                p.lock()
                    .map(|s| format!("Publisher({})", s.get_topic()))
                    .unwrap_or("Unknown".into())
            })
            .unwrap_or("Unknown".into());

        Self {
            topic: value.topic,
            subscriber,
            publisher,
            latencies: value.latencies,
        }
    }
}

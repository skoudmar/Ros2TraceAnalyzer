#![forbid(unsafe_code, reason = "It shoudn't be needed")]

mod analysis;
mod events_common;
mod model;
mod processed_events;
mod processor;
mod raw_events;
mod utils;

use std::env;

use bt2_sys::iterator::MessageIterator;
use bt2_sys::message::BtMessageType;

struct ProcessedEventsIter<'a> {
    iter: MessageIterator,
    on_unprocessed_event: fn(raw_events::FullEvent),
    analyses: Vec<&'a mut dyn analysis::EventAnalysis>,
    processor: processor::Processor,
}

impl<'a> ProcessedEventsIter<'a> {
    fn new(trace_path: &str) -> Self {
        Self {
            iter: MessageIterator::new(trace_path),
            on_unprocessed_event: |event| {
                println!("Unprocessed event: {event:?}");
            },
            analyses: Vec::new(),
            processor: processor::Processor::new(),
        }
    }

    fn add_analysis(&mut self, analysis: &'a mut dyn analysis::EventAnalysis) {
        analysis.initialize();
        self.analyses.push(analysis);
    }

    fn set_on_unprocessed_event(&mut self, on_unprocessed_event: fn(raw_events::FullEvent)) {
        self.on_unprocessed_event = on_unprocessed_event;
    }
}

impl<'a> Iterator for ProcessedEventsIter<'a> {
    type Item = processed_events::FullEvent;

    fn next(&mut self) -> Option<Self::Item> {
        for message in self.iter.by_ref() {
            let event = match message.get_type() {
                BtMessageType::StreamBeginning
                | BtMessageType::StreamEnd
                | BtMessageType::PacketBeginning
                | BtMessageType::PacketEnd => {
                    // Silently skip these messages
                    continue;
                }
                BtMessageType::DiscardedEvents
                | BtMessageType::DiscardedPackets
                | BtMessageType::MessageIteratorInactivity => {
                    println!("Skipping message of type {:?}", message.get_type());
                    continue;
                }
                BtMessageType::Event => raw_events::get_full_event(&message.into_event_msg()),
            };

            let Some(event) = event else {
                // Skip unsupported events
                continue;
            };
            match self.processor.process_raw_event(event) {
                processor::MaybeProccessed::Processed(proccessed) => {
                    for analysis in &mut self.analyses {
                        (*analysis).process_event(&proccessed);
                    }
                    return Some(proccessed);
                }
                processor::MaybeProccessed::Raw(raw) => {
                    (self.on_unprocessed_event)(raw);
                    continue;
                }
            }
        }

        for analysis in &mut self.analyses {
            analysis.finalize();
        }

        None
    }
}

fn main() {
    let trace_path = env::args()
        .nth(1)
        .expect("Expected trace path as first argument");

    let mut message_latency_analysis = analysis::MessageLatency::new();
    let mut callback_duration_analysis = analysis::CallbackDuration::new();

    let mut iter = ProcessedEventsIter::new(&trace_path);
    iter.add_analysis(&mut message_latency_analysis);
    iter.add_analysis(&mut callback_duration_analysis);
    iter.set_on_unprocessed_event(|event| {
        println!("Unprocessed event: {event:?}");
    });

    for event in &mut iter {
        println!("{event}");
    }

    iter.processor.print_objects();

    message_latency_analysis.print_stats();
    callback_duration_analysis.print_stats();
}

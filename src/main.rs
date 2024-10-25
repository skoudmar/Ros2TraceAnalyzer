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

fn main() {
    let trace_path = env::args()
        .nth(1)
        .expect("Expected trace path as first argument");

    let mut processor = processor::Processor::new();

    for message in MessageIterator::new(&trace_path).take(1_000_000) {
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
            continue;
        };
        match processor.process_raw_event(event) {
            Ok(processed_event) => {
                println!("{processed_event}");
            }
            Err(e) => {
                println!("unprocessed event: {e:?}");
            }
        }
    }
}

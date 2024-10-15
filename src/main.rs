mod model;
mod raw_events;
mod utils;
mod processor;

use std::env;

use bt2_sys::{iterator::MessageIterator, message::BtMessageType};

fn main() {
    let trace_path = env::args()
        .nth(1)
        .expect("Expected trace path as first argument");

    for message in MessageIterator::new(&trace_path).take(1_000_000) {
        if message.get_type() != BtMessageType::Event {
            println!("Skipping message of type {:?}", message.get_type());
            continue;
        }

        assert!(matches!(message.get_type(), BtMessageType::Event));
        let Some(event) = raw_events::get_full_event(&message) else {
            continue;
        };
        println!("{event:?}");
    }
}

use std::cmp::Reverse;
use std::collections::HashMap;
use std::env;
use std::ffi::CString;

use bt2_sys::iterator::MessageIterator;
use bt2_sys::message::BtMessageType;

fn main() {
    let trace_path = env::args()
        .nth(1)
        .expect("Usage: ./count_events <trace-path>");
    let trace_path_cstring = CString::new(trace_path).unwrap();

    let message_iterator = MessageIterator::new(&[&trace_path_cstring]);

    let mut counter = HashMap::new();
    let mut total_events = 0;

    for message in message_iterator.filter(|msg| msg.get_type() == BtMessageType::Event) {
        let event_msg = message.into_event_msg();
        let event = event_msg.get_event();
        let event_class = event.get_class();
        let event_name = event_class.get_name().unwrap();

        *counter.entry(event_name.to_owned()).or_insert(0) += 1;
        total_events += 1;
    }

    let mut counter = counter.into_iter().collect::<Vec<_>>();
    counter.sort_by_key(|(_, count)| Reverse(*count));

    println!("Total events: {total_events}");
    println!("Event counts by name:");
    for (event_name, count) in counter {
        println!("{event_name} {count}");
    }
}

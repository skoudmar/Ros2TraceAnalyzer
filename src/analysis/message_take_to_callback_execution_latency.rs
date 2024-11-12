use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::analysis::utils::calculate_min_max_avg;
use crate::model::display::DisplayCallbackSummary;
use crate::model::{Callback, CallbackInstance, CallbackTrigger};
use crate::processed_events::{ros2, Event, FullEvent};
use crate::utils::DurationDisplayImprecise;

use super::{ArcMutWrapper, EventAnalysis};

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
        println!("Message take to callback execution latency statistics:");
        for (i, (callback, latencies)) in self.latencies.iter().enumerate() {
            let callback = callback.0.lock().unwrap();

            println!("- [{i:4}] Callback {}:", DisplayCallbackSummary(&callback));
            println!("    Call count: {}", latencies.len());
            if let Some((min_latency, max_latency, avg_latency)) = calculate_min_max_avg(latencies)
            {
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

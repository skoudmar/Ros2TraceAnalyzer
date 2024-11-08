use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::model::{Callback, CallbackInstance};
use crate::processed_events::{ros2, Event, FullEvent};
use crate::utils::DurationDisplayImprecise;

use super::{ArcMutWrapper, EventAnalysis};

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

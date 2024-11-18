use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::model::display::{get_node_name_from_weak, DisplayCallbackSummary};
use crate::model::{Callback, CallbackInstance};
use crate::processed_events::{ros2, Event, FullEvent};
use crate::statistics::calculate_min_max_avg;
use crate::utils::{DurationDisplayImprecise, WeakKnown};

use super::{AnalysisOutput, ArcMutWrapper, EventAnalysis};

#[derive(Debug)]
pub struct CallbackDuration {
    durations: HashMap<ArcMutWrapper<Callback>, Vec<i64>>,
    started_callbacks: HashSet<ArcMutWrapper<CallbackInstance>>,
    not_ended_callbacks: Vec<ArcMutWrapper<CallbackInstance>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Record {
    node: String,
    callback_type: String,
    callback_caller: String,

    call_count: usize,
    max_duration: i64,
    min_duration: i64,
    avg_duration: i64,
}

#[derive(Debug, Clone)]
pub struct RecordSummary {
    pub(crate) call_count: usize,
    pub(crate) max_duration: i64,
    pub(crate) min_duration: i64,
    pub(crate) avg_duration: i64,
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

    fn calculate_duration_summary(
        &self,
        callback: &ArcMutWrapper<Callback>,
    ) -> Option<RecordSummary> {
        let durations = self.durations.get(callback)?;
        debug_assert!(
            !durations.is_empty(),
            "Callback should have at least one duration"
        );

        let dur_len = durations.len();
        let (min_duration, max_duration, avg_duration) = calculate_min_max_avg(durations)?;

        Some(RecordSummary {
            call_count: dur_len,
            max_duration,
            min_duration,
            avg_duration,
        })
    }

    pub fn get_records(&self) -> Vec<Record> {
        self.durations
            .keys()
            .map(|callback_arc| {
                let callback = callback_arc.0.lock().unwrap();
                let node_name = callback.get_node().map_or(WeakKnown::Unknown, |node_weak| {
                    get_node_name_from_weak(&node_weak.get_weak())
                });
                let callback_type = callback.get_type();
                let callback_caller = callback.get_caller().unwrap().get_caller_as_string();
                drop(callback);

                let summary = self
                    .calculate_duration_summary(callback_arc)
                    .expect("Callback key should exist.");

                Record {
                    node: node_name.to_string(),
                    callback_type: callback_type.to_string(),
                    callback_caller: callback_caller.to_string(),
                    call_count: summary.call_count,
                    max_duration: summary.max_duration,
                    min_duration: summary.min_duration,
                    avg_duration: summary.avg_duration,
                }
            })
            .collect()
    }

    pub(crate) fn print_stats(&self) {
        println!("Callback duration statistics:");
        for (i, callback_arc) in self.durations.keys().enumerate() {
            let callback = callback_arc.0.lock().unwrap();
            let summary = self
                .calculate_duration_summary(callback_arc)
                .expect("Callback key should exist.");

            println!("- [{i:4}] Callback {}:", DisplayCallbackSummary(&callback));
            println!("    Call count: {}", summary.call_count);
            if summary.call_count > 0 {
                println!(
                    "    Max duration: {}",
                    DurationDisplayImprecise(summary.max_duration)
                );
                println!(
                    "    Min duration: {}",
                    DurationDisplayImprecise(summary.min_duration)
                );
                println!(
                    "    Avg duration: {}",
                    DurationDisplayImprecise(summary.avg_duration)
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

impl AnalysisOutput for CallbackDuration {
    const FILE_NAME: &'static str = "callback_duration";

    fn write_csv(&self, writer: &mut csv::Writer<std::fs::File>) -> csv::Result<()> {
        let records = self.get_records();
        for record in records {
            writer.serialize(record)?;
        }

        Ok(())
    }
}

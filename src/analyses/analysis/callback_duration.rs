use serde::{Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crate::events_common::Context;
use crate::model::display::get_node_name_from_weak;
use crate::model::{Callback, CallbackInstance, Time};
use crate::processed_events::{Event, FullEvent, ros2};
use crate::utils::WeakKnown;

use super::{AnalysisOutput, ArcMutWrapper, EventAnalysis};

#[derive(Debug, Serialize)]
pub struct ExecutionData {
    #[serde(serialize_with = "serialize_time")]
    pub start_time: Time,
    pub duration: i64,
    pub tid: u32,
    pub cpuid: u32,
}

fn serialize_time<S>(time: &Time, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_i64(time.timestamp_nanos())
}

#[derive(Debug, Default)]
pub struct CallbackDuration {
    execution_data: HashMap<ArcMutWrapper<Callback>, Vec<ExecutionData>>,
    // durations: HashMap<ArcMutWrapper<Callback>, Vec<i64>>,
    // execution_start_time: HashMap<ArcMutWrapper<Callback>, Vec<Time>>,
    started_callbacks: HashSet<ArcMutWrapper<CallbackInstance>>,
    not_ended_callbacks: Vec<ArcMutWrapper<CallbackInstance>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Record {
    node: String,
    caller_type: String,
    /// The caller parameter is the main parameter of the caller.
    ///
    /// - For subscriptions, it is the topic name.
    /// - For timers, it is the timer period.
    /// - For services, it is the service name.
    caller_param: String,

    durations: Vec<i64>,
    inter_arrival_times: Vec<i64>,
}

impl CallbackDuration {
    pub fn new() -> Self {
        Self {
            execution_data: HashMap::new(),
            // durations: HashMap::new(),
            // execution_start_time: HashMap::new(),
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

    fn end_callback(&mut self, callback: Arc<Mutex<CallbackInstance>>, context: &Context) {
        let callback = callback.into();
        if self.started_callbacks.remove(&callback) {
            let callback_instance = callback.0.lock().unwrap();
            let duration = Self::calculate_duration(&callback_instance)
                .expect("Duration should be known in callback_end");

            // self.durations
            //     .entry(callback_instance.get_callback().into())
            //     .or_default()
            //     .push(duration);
            // self.execution_start_time
            //     .entry(callback_instance.get_callback().into())
            //     .or_default()
            //     .push(callback_instance.get_start_time());

            self.execution_data
                .entry(callback_instance.get_callback().into())
                .or_default()
                .push(ExecutionData {
                    start_time: callback_instance.get_start_time(),
                    duration,
                    tid: context.vtid(),
                    cpuid: context.cpu_id(),
                });
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

    pub fn get_records(&self) -> Vec<Record> {
        self.execution_data
            .iter()
            .map(|(callback_arc, data)| {
                let callback = callback_arc.0.lock().unwrap();
                let node_name = callback.get_node().map_or(WeakKnown::Unknown, |node_weak| {
                    get_node_name_from_weak(&node_weak.get_weak())
                });
                let callback_type = callback.get_type();
                let callback_caller = callback.get_caller().unwrap().get_caller_as_string();
                drop(callback);

                Record {
                    node: node_name.to_string(),
                    caller_type: callback_type.to_string(),
                    caller_param: callback_caller.to_string(),
                    durations: data.iter().map(|data| data.duration).collect(),
                    inter_arrival_times: Self::get_inter_arrival_time_inner(data)
                        .unwrap_or_default(),
                }
            })
            .collect()
    }

    fn get_inter_arrival_time_inner(data: &[ExecutionData]) -> Option<Vec<i64>> {
        if data.len() < 2 {
            return None;
        }

        let mut inter_callback_time = Vec::with_capacity(data.len() - 1);

        for w in data.windows(2) {
            let (first_time, second_time) = (w[0].start_time, w[1].start_time);
            assert!(second_time >= first_time);
            let duration = second_time.timestamp_nanos() - first_time.timestamp_nanos();
            inter_callback_time.push(duration);
        }

        Some(inter_callback_time)
    }

    pub(super) fn get_inter_arrival_time(
        &self,
        callback: &ArcMutWrapper<Callback>,
    ) -> Option<Vec<i64>> {
        let start_times = self.execution_data.get(callback)?;
        Self::get_inter_arrival_time_inner(start_times)
    }

    pub(super) fn get_execution_data(
        &self,
    ) -> &HashMap<ArcMutWrapper<Callback>, Vec<ExecutionData>> {
        &self.execution_data
    }
}

impl EventAnalysis for CallbackDuration {
    fn initialize(&mut self) {
        self.execution_data.clear();
        self.started_callbacks.clear();
        self.not_ended_callbacks.clear();
    }

    fn process_event(&mut self, full_event: &FullEvent) {
        match &full_event.event {
            Event::Ros2(ros2::Event::CallbackStart(event)) => {
                self.start_callback(event.callback.clone());
            }
            Event::Ros2(ros2::Event::CallbackEnd(event)) => {
                self.end_callback(event.callback.clone(), &full_event.context);
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
    fn write_json(&self, file: &mut std::io::BufWriter<std::fs::File>) -> serde_json::Result<()> {
        let records = self.get_records();
        serde_json::to_writer(file, &records)
    }
}

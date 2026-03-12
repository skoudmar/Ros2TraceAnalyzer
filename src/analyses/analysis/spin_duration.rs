use std::collections::HashMap;

use crate::analysis::utils::DisplayDurationStats;
use crate::model::Node;
use crate::processed_events::{self, Event};
use crate::utils::DurationDisplayImprecise;

use super::{AnalysisOutput, ArcMutWrapper, EventAnalysis};

#[derive(Debug, Default)]
pub struct SpinDuration {
    processing_durations: HashMap<ArcMutWrapper<Node>, Vec<i64>>,
}

impl SpinDuration {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn print_stats(&self) {
        println!("Spin duration statistics:");
        for (i, (node, durations)) in self.processing_durations.iter().enumerate() {
            let node = node.0.lock().unwrap();

            println!("- [{i:4}] Node {}:", node.get_full_name());
            println!("    Call count: {}", durations.len());
            let display = DisplayDurationStats::with_comma(durations);
            println!("    Duration: {display}");
            let (mean, std_dev) = display.mean_and_std_dev();
            println!(
                "    Mean: {}, Std. dev.: {}",
                DurationDisplayImprecise(mean),
                DurationDisplayImprecise(std_dev as i64)
            );
        }
    }
}

impl EventAnalysis for SpinDuration {
    fn initialize(&mut self) {
        self.processing_durations.clear();
    }

    fn process_event(&mut self, full_event: &crate::processed_events::FullEvent) {
        if let Event::R2r(processed_events::r2r::Event::SpinEnd(event)) = &full_event.event {
            let node = event.node.clone();
            let spin = event.spin.as_ref().unwrap();
            let spin_guard = spin.lock().unwrap();
            let start_time = spin_guard
                .get_wake_time()
                .expect("Bug: Spin end event without wake time set");
            let end_time = spin_guard
                .get_end_time()
                .expect("Bug: Spin end event without end time set");
            let duration = end_time.timestamp_nanos() - start_time.timestamp_nanos();

            self.processing_durations
                .entry(node.into())
                .or_default()
                .push(duration);
        }
    }

    fn finalize(&mut self) {}
}

#[derive(Debug, serde::Serialize)]
struct SpinDurationEntry {
    node: String,
    spin_duration: Vec<i64>,
}

impl AnalysisOutput for SpinDuration {
    fn write_json(&self, file: &mut std::io::BufWriter<std::fs::File>) -> serde_json::Result<()> {
        let spin_durations: Vec<SpinDurationEntry> = self
            .processing_durations
            .iter()
            .map(|(node, durations)| SpinDurationEntry {
                node: node.0.lock().unwrap().get_full_name().unwrap().to_owned(),
                spin_duration: durations.clone(),
            })
            .collect();

        serde_json::to_writer(file, &spin_durations)
    }
}

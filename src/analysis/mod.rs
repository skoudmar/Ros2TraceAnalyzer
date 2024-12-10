use std::fs::File;
use std::hash::Hash;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::processed_events::FullEvent;
use derive_more::derive::From;

mod utils;

pub mod dependency_graph;
pub use dependency_graph::DependencyGraph;

pub mod message_latency;
pub use message_latency::MessageLatency;

pub mod callback_duration;
pub use callback_duration::CallbackDuration;

pub mod publication_in_callback;
pub use publication_in_callback::PublicationInCallback;

pub mod callback_dependency;
pub use callback_dependency::CallbackDependency;

pub mod message_take_to_callback_execution_latency;
pub use message_take_to_callback_execution_latency::MessageTakeToCallbackLatency;

pub mod utilization;
pub use utilization::Utilization;

pub trait EventAnalysis {
    /// Initialize the analysis
    ///
    /// This method is called before any events are processed
    fn initialize(&mut self);

    /// Process an event
    fn process_event(&mut self, event: &FullEvent);

    /// Finalize the analysis
    ///
    /// This method is called after all events have been processed
    fn finalize(&mut self);
}

pub trait AnalysisOutput {
    const FILE_NAME: &'static str;

    fn write_json(&self, file: &mut File) -> serde_json::Result<()>;
}

pub trait AnalysisOutputExt: AnalysisOutput {
    fn write_json_to_output_dir(&self, output_dir: &Path) -> std::io::Result<()> {
        let out_file = output_dir.join(Self::FILE_NAME).with_extension("json");
        let mut out_file = File::create(out_file)?;
        self.write_json(&mut out_file).map_err(Into::into)
    }
}

impl<T: AnalysisOutput> AnalysisOutputExt for T {}

#[derive(Debug, From)]
struct ArcMutWrapper<T>(Arc<Mutex<T>>);

impl<T> PartialEq for ArcMutWrapper<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Clone for ArcMutWrapper<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Eq for ArcMutWrapper<T> {}

impl<T> Hash for ArcMutWrapper<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

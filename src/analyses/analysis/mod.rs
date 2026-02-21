use std::fs::File;
use std::hash::Hash;
use std::io::BufWriter;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::processed_events::FullEvent;
use crate::utils::binary_sql_store::{BinarySQLStore, BinarySQLStoreError};
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

pub mod spin_duration;
pub use spin_duration::SpinDuration;

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
    fn write_json(&self, file: &mut BufWriter<File>) -> serde_json::Result<()>;

    fn get_binary_output(&self) -> impl serde::Serialize;
}

pub trait AnalysisOutputExt: AnalysisOutput {
    fn write_json_to_output_dir(&self, path: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(path.parent().unwrap())?;
        let out_file = File::create(path)?;
        let mut out_file = BufWriter::new(out_file);
        self.write_json(&mut out_file).map_err(Into::into)
    }

    fn write_to_binary(
        &self,
        store: &BinarySQLStore,
        store_name: &str,
    ) -> Result<(), BinarySQLStoreError> {
        let data = self.get_binary_output();
        store.write(store_name, data)
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

use std::hash::Hash;
use std::sync::{Arc, Mutex};

use crate::processed_events::FullEvent;
use derive_more::derive::From;

pub mod message_latency;
pub use message_latency::MessageLatency;

pub mod callabck_duration;
pub use callabck_duration::CallbackDuration;

pub mod publication_in_callback;
pub use publication_in_callback::PublicationInCallback;

pub mod callback_dependency;
pub use callback_dependency::CallbackDependency;

pub mod message_take_to_callback_execution_latency;
pub use message_take_to_callback_execution_latency::MessageTakeToCallbackLatency;

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

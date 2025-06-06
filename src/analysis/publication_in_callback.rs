use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::events_common::Context;
use crate::model::display::DisplayCallbackSummary;
use crate::model::{Callback, CallbackInstance, PublicationMessage, Publisher};
use crate::processed_events::{ros2, Event, FullEvent};
use crate::utils::DisplayDebug;

use super::{ArcMutWrapper, EventAnalysis};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Id {
    vtid: u32,
    hostname: String,
}

#[derive(Debug, Default)]
pub struct PublicationInCallback {
    active_callbacks: HashMap<Id, ArcMutWrapper<CallbackInstance>>,
    dependency: HashSet<(ArcMutWrapper<Publisher>, ArcMutWrapper<Callback>)>,
}

impl PublicationInCallback {
    pub fn new() -> Self {
        Self::default()
    }

    fn activate_callback(&mut self, callback: Arc<Mutex<CallbackInstance>>, context: &Context) {
        let id = Id {
            vtid: context.vtid(),
            hostname: context.hostname().to_string(),
        };

        if let Some(old) = self.active_callbacks.insert(id, callback.into()) {
            panic!(
                "Callback {old:?} is already active on vtid {} on host {}",
                context.vtid(),
                context.hostname()
            );
        }
    }

    fn deactivate_callback(&mut self, callback: &Arc<Mutex<CallbackInstance>>, context: &Context) {
        let id = Id {
            vtid: context.vtid(),
            hostname: context.hostname().to_string(),
        };

        if let Some(old) = self.active_callbacks.remove(&id) {
            assert!(
                Arc::ptr_eq(&old.0, callback),
                "Callback {old:?} is not active on vtid {} on host {}",
                context.vtid(),
                context.hostname(),
            );
        } else {
            panic!(
                "No callback is being executed on vtid {} on host {}",
                context.vtid(),
                context.hostname(),
            );
        }
    }

    fn process_publication(
        &mut self,
        publication: Arc<Mutex<PublicationMessage>>,
        context: &Context,
    ) {
        let id = Id {
            vtid: context.vtid(),
            hostname: context.hostname().to_string(),
        };

        if let Some(callback) = self.active_callbacks.get(&id) {
            let callback = callback.0.lock().unwrap();
            let publication_msg = publication.lock().unwrap();
            let publisher_arc = publication_msg.get_publisher().unwrap();

            self.dependency
                .insert((publisher_arc.into(), callback.get_callback().into()));
        }
    }

    pub(super) const fn get_dependency(
        &self,
    ) -> &HashSet<(ArcMutWrapper<Publisher>, ArcMutWrapper<Callback>)> {
        &self.dependency
    }

    pub(crate) fn write_stats(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        writeln!(output, "Publication in callbacks statistic:")?;
        for (i, (publisher, callback)) in self.dependency.iter().enumerate() {
            let publisher = publisher.0.lock().unwrap();
            let callback = callback.0.lock().unwrap();
            writeln!(
                output,
                "- [{i:4}] Callback {}\n\t-> Publisher {}",
                DisplayCallbackSummary(&callback),
                DisplayDebug(publisher.get_topic())
            )?;
        }
        Ok(())
    }
}

impl EventAnalysis for PublicationInCallback {
    fn initialize(&mut self) {
        *self = Self::default();
    }

    fn process_event(&mut self, full_event: &FullEvent) {
        match &full_event.event {
            Event::Ros2(ros2::Event::CallbackStart(event)) => {
                self.activate_callback(event.callback.clone(), &full_event.context);
            }
            Event::Ros2(ros2::Event::CallbackEnd(event)) => {
                self.deactivate_callback(&event.callback, &full_event.context);
            }
            Event::Ros2(ros2::Event::RmwPublish(event)) => {
                self.process_publication(event.message.clone(), &full_event.context);
            }
            _ => {}
        }
    }

    fn finalize(&mut self) {
        // Nothing to do
    }
}

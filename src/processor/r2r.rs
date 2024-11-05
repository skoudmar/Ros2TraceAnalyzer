use color_eyre::eyre::OptionExt;

use crate::events_common::Context;
use crate::model::{SpinInstance, Time};
use crate::{processed_events, raw_events};

use super::{ContextId, IntoId};

impl super::Processor {
    pub fn process_raw_r2r_event(
        &mut self,
        event: raw_events::r2r::Event,
        context: &Context,
        time: Time,
    ) -> Result<processed_events::r2r::Event, raw_events::r2r::Event> {
        let context_id = ContextId::new(context.vpid(), self.host_to_host_id(context.hostname()));

        Ok(match event {
            raw_events::r2r::Event::SpinStart(event) => {
                self.process_spin_start(event, context_id, time).into()
            }
            raw_events::r2r::Event::SpinEnd(event) => {
                self.process_spin_end(event, context_id, time).into()
            }
            raw_events::r2r::Event::SpinWake(event) => {
                self.process_spin_wake(event, context_id, time).into()
            }
            raw_events::r2r::Event::SpinTimeout(event) => {
                self.process_spin_timeout(event, context_id, time).into()
            }
            raw_events::r2r::Event::UpdateTime(event) => {
                self.process_update_time(event, context_id, time).into()
            }
        })
    }

    fn process_spin_start(
        &mut self,
        event: raw_events::r2r::SpinStart,
        context_id: ContextId,
        time: Time,
    ) -> processed_events::r2r::SpinStart {
        let timeout = std::time::Duration::new(event.timeout_s, event.timeout_ns);

        let node = self
            .nodes_by_rcl
            .get(&event.node_handle.into_id(context_id))
            .unwrap()
            .clone();

        let spin_instance = SpinInstance::new(&node, time, timeout);

        processed_events::r2r::SpinStart {
            node,
            spin: spin_instance,
        }
    }

    fn process_spin_end(
        &mut self,
        event: raw_events::r2r::SpinEnd,
        context_id: ContextId,
        time: Time,
    ) -> processed_events::r2r::SpinEnd {
        let node = self
            .nodes_by_rcl
            .get(&event.node_handle.into_id(context_id))
            .unwrap()
            .clone();

        let spin = node
            .lock()
            .unwrap()
            .take_spin_instance()
            .inspect(|spin_arc| {
                let mut spin = spin_arc.lock().unwrap();
                spin.set_end_time(time);
            });

        processed_events::r2r::SpinEnd { node, spin }
    }

    fn process_spin_wake(
        &mut self,
        event: raw_events::r2r::SpinWake,
        context_id: ContextId,
        time: Time,
    ) -> processed_events::r2r::SpinWake {
        let node_arc = self
            .nodes_by_rcl
            .get(&event.node_handle.into_id(context_id))
            .unwrap()
            .clone();

        let node = node_arc.lock().unwrap();
        let spin_arc = node.borrow_spin_instance();
        let spin = spin_arc.cloned().inspect(|spin_arc| {
            let mut spin = spin_arc.lock().unwrap();
            spin.set_wake_time(time);
        });
        drop(node);

        processed_events::r2r::SpinWake {
            node: node_arc,
            spin,
        }
    }

    fn process_spin_timeout(
        &mut self,
        event: raw_events::r2r::SpinTimeout,
        context_id: ContextId,
        time: Time,
    ) -> processed_events::r2r::SpinTimeout {
        let node_arc = self
            .nodes_by_rcl
            .get(&event.node_handle.into_id(context_id))
            .unwrap()
            .clone();

        let mut node = node_arc.lock().unwrap();
        let spin_arc = node.take_spin_instance();
        let spin = spin_arc.inspect(|spin_arc| {
            let mut spin = spin_arc.lock().unwrap();
            spin.set_timeouted_on(time);
        });
        drop(node);

        processed_events::r2r::SpinTimeout {
            node: node_arc,
            spin,
        }
    }

    fn process_update_time(
        &mut self,
        event: raw_events::r2r::UpdateTime,
        context_id: ContextId,
        _time: Time,
    ) -> processed_events::r2r::UpdateTime {
        let subscriber_arc = self
            .subscribers_by_rcl
            .get(&event.subscriber.into_id(context_id))
            .ok_or_eyre("Subscriber not found for update_time event")
            .unwrap()
            .clone();

        processed_events::r2r::UpdateTime {
            subscriber: subscriber_arc,
            time_s: event.time_s,
            time_ns: event.time_ns,
        }
    }
}

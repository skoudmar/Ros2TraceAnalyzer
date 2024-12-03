use color_eyre::eyre::Context as _;
use color_eyre::Result;

use crate::events_common::Context;
use crate::model::{SpinInstance, Time};
use crate::{processed_events, raw_events};

use super::{ContextId, IntoId, MapGetAsResult, UnsupportedOrError};

impl super::Processor {
    pub fn process_raw_r2r_event(
        &mut self,
        event: raw_events::r2r::Event,
        context: &Context,
        time: Time,
    ) -> Result<processed_events::r2r::Event, UnsupportedOrError<raw_events::r2r::Event>> {
        let context_id = ContextId::new(context.vpid(), self.host_to_host_id(context.hostname()));

        Ok(match event {
            raw_events::r2r::Event::SpinStart(event) => self
                .process_spin_start(event, time, context_id, context)?
                .into(),
            raw_events::r2r::Event::SpinEnd(event) => self
                .process_spin_end(event, time, context_id, context)?
                .into(),
            raw_events::r2r::Event::SpinWake(event) => self
                .process_spin_wake(event, time, context_id, context)?
                .into(),
            raw_events::r2r::Event::SpinTimeout(event) => self
                .process_spin_timeout(event, time, context_id, context)?
                .into(),
            raw_events::r2r::Event::UpdateTime(event) => self
                .process_update_time(event, time, context_id, context)?
                .into(),
        })
    }

    fn process_spin_start(
        &mut self,
        event: raw_events::r2r::SpinStart,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::r2r::SpinStart> {
        let timeout = std::time::Duration::new(event.timeout_s, event.timeout_ns);

        let node_arc = self
            .nodes_by_rcl
            .get_or_err(event.node_handle.into_id(context_id), "rcl_handle")
            .map_err(|e| e.with_r2r_event(&event, time, context))?
            .clone();

        let spin_instance = SpinInstance::new(&node_arc, time, timeout);
        {
            let mut node = node_arc.lock().unwrap();
            let old = node.replace_spin_instance(spin_instance.clone());

            if let Some(old) = old {
                log::warn!(target:"r2r::spin_start","Node {node} has a spin instance already set: {old:?}");
            }
        }

        Ok(processed_events::r2r::SpinStart {
            node: node_arc,
            spin: spin_instance,
        })
    }

    fn process_spin_end(
        &mut self,
        event: raw_events::r2r::SpinEnd,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::r2r::SpinEnd> {
        let node_arc = self
            .nodes_by_rcl
            .get_or_err(event.node_handle.into_id(context_id), "rcl_handle")
            .map_err(|e| e.with_r2r_event(&event, time, context))?
            .clone();

        let spin = node_arc
            .lock()
            .unwrap()
            .take_spin_instance()
            .inspect(|spin_arc| {
                let mut spin = spin_arc.lock().unwrap();
                spin.set_end_time(time);
            });

        Ok(processed_events::r2r::SpinEnd {
            node: node_arc,
            spin,
        })
    }

    fn process_spin_wake(
        &mut self,
        event: raw_events::r2r::SpinWake,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::r2r::SpinWake> {
        let node_arc = self
            .nodes_by_rcl
            .get_or_err(event.node_handle.into_id(context_id), "rcl_handle")
            .map_err(|e| e.with_r2r_event(&event, time, context))?
            .clone();

        let node = node_arc.lock().unwrap();
        let spin_arc = node.borrow_spin_instance();
        let spin = spin_arc.cloned().inspect(|spin_arc| {
            let mut spin = spin_arc.lock().unwrap();
            spin.set_wake_time(time);
        });
        drop(node);

        Ok(processed_events::r2r::SpinWake {
            node: node_arc,
            spin,
        })
    }

    fn process_spin_timeout(
        &mut self,
        event: raw_events::r2r::SpinTimeout,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::r2r::SpinTimeout> {
        let node_arc = self
            .nodes_by_rcl
            .get_or_err(event.node_handle.into_id(context_id), "rcl_handle")
            .map_err(|e| e.with_r2r_event(&event, time, context))?
            .clone();

        let mut node = node_arc.lock().unwrap();
        let spin_arc = node.take_spin_instance();
        let spin = spin_arc.inspect(|spin_arc| {
            let mut spin = spin_arc.lock().unwrap();
            spin.set_timeouted_on(time);
        });
        drop(node);

        Ok(processed_events::r2r::SpinTimeout {
            node: node_arc,
            spin,
        })
    }

    fn process_update_time(
        &mut self,
        event: raw_events::r2r::UpdateTime,
        time: Time,
        context_id: ContextId,
        context: &Context,
    ) -> Result<processed_events::r2r::UpdateTime> {
        let subscriber_arc = self
            .subscribers_by_rcl
            .get_or_err(event.subscriber.into_id(context_id), "rcl_handle")
            .map_err(|e| e.with_r2r_event(&event, time, context))
            .wrap_err("Subscriber not found for update_time event")?
            .clone();

        Ok(processed_events::r2r::UpdateTime {
            subscriber: subscriber_arc,
            time_s: event.time_s,
            time_ns: event.time_ns,
        })
    }
}

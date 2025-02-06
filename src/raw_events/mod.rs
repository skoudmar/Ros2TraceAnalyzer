use std::fmt::{Debug, Display};

use bt2_sys::event::BtEventConst;
use bt2_sys::field::BtFieldConst;
use bt2_sys::message::BtEventMessageConst;
use bt2_sys::trace::BtEnvironmentEntry;
use derive_more::derive::From;

use crate::events_common::{Context, Time};

pub mod ros2;

pub mod r2r;

pub trait FromBtEvent: Sized {
    fn from_event(event: &BtEventConst) -> Option<Self>;
}

impl<T> FromBtEvent for T
where
    T: TryFrom<BtFieldConst>,
    <T as TryFrom<BtFieldConst>>::Error: Display,
{
    fn from_event(event: &BtEventConst) -> Option<Self> {
        Some(event.get_payload()?.try_into().unwrap_or_else(|e| {
            panic!(
                "Failed to parse event payload into {}: {e}",
                std::any::type_name::<T>()
            )
        }))
    }
}

pub fn time_from_message(message: &BtEventMessageConst) -> Time {
    let clock_snapshot = message.get_default_clock_snapshot();
    let value_from_origin = clock_snapshot.get_value_from_origin().unwrap();
    Time::from_nanos(value_from_origin)
}

pub fn context_from_event(event: &BtEventConst) -> Context {
    let cpu_id = event
        .get_packet()
        .get_context_field()
        .unwrap_or_else(|| panic!("Missing context field in event {event:?}"))
        .into_struct()
        .get_field_by_name_cstr(c"cpu_id")
        .unwrap_or_else(|| panic!("Missing cpu_id field in event {event:?}"))
        .into_uint()
        .get_value()
        .try_into()
        .unwrap();

    let trace = event.get_stream().get_trace();
    let BtEnvironmentEntry::String(hostname) = trace
        .get_environment_entry_by_name_cstr(c"hostname")
        .expect("Trace missing hostname environment entry")
    else {
        panic!("Missing hostname environment entry");
    };
    let hostname = hostname
        .get()
        .expect("Hostname should be valid UTF-8.")
        .to_string();

    let common_context = event
        .get_common_context_field()
        .expect("Event missing common context field")
        .into_struct();
    let vpid = common_context
        .get_field_by_name_cstr(c"vpid")
        .expect("Missing vpid")
        .into_int()
        .get_value()
        .try_into()
        .unwrap();
    let vtid = common_context
        .get_field_by_name_cstr(c"vtid")
        .expect("Missing vtid")
        .into_int()
        .get_value()
        .try_into()
        .unwrap();
    let procname = common_context
        .get_field_by_name_cstr(c"procname")
        .expect("Missing procname")
        .into_string()
        .get_value()
        .to_string();
    Context::new(cpu_id, vpid, vtid, procname, hostname)
}

#[derive(Debug, Clone)]
pub struct FullEvent {
    pub context: Context,
    pub time: Time,
    pub event: Event,
}

#[derive(Debug, From, Clone)]
pub enum Event {
    Ros2(ros2::Event),
    R2r(r2r::Event),
}

pub fn get_full_event(message: &BtEventMessageConst) -> Option<FullEvent> {
    let bt_event = message.get_event();

    let bt_event_class = bt_event.get_class();
    let full_name = bt_event_class.get_name().unwrap();
    let provider = full_name.split(':').next().unwrap();

    let event = match provider {
        "ros2" => Event::Ros2(ros2::Event::from_event(&bt_event)?),
        "r2r" => Event::R2r(r2r::Event::from_event(&bt_event)?),
        _ => {
            return None;
        }
    };

    let context = context_from_event(&bt_event);
    let time = time_from_message(message);

    Some(FullEvent {
        context,
        time,
        event,
    })
}

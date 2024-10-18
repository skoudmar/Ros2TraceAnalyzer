use std::fmt::Debug;

use bt2_sys::{event::BtEventConst, message::BtMessageConst, trace::BtEnvironmentEntry};

use crate::{
    events_common::{Context, Time},
    impl_from_for_enum,
};

pub mod ros2;

pub mod r2r;

pub trait FromBtEvent: Sized {
    fn from_event(event: &BtEventConst) -> Option<Self>;
}

pub fn time_from_message(message: &BtMessageConst) -> Time {
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
        .get_field_by_name("cpu_id")
        .unwrap_or_else(|| panic!("Missing cpu_id field in event {event:?}"))
        .into_uint()
        .get_value()
        .try_into()
        .unwrap();

    let BtEnvironmentEntry::String(hostname) = event
        .get_stream()
        .get_trace()
        .get_environment_entry_by_name("hostname")
        .expect("Trace missing hostname environment entry")
    else {
        panic!("Missing hostname environment entry");
    };
    let hostname = hostname.get_value().to_string();

    let common_context = event
        .get_common_context_field()
        .expect("Event missing common context field")
        .into_struct();
    let vpid = common_context
        .get_field_by_name("vpid")
        .expect("Missing vpid")
        .into_int()
        .get_value()
        .try_into()
        .unwrap();
    let vtid = common_context
        .get_field_by_name("vtid")
        .expect("Missing vtid")
        .into_int()
        .get_value()
        .try_into()
        .unwrap();
    let procname = common_context
        .get_field_by_name("procname")
        .expect("Missing procname")
        .into_string()
        .get_value()
        .to_string();
    Context::new(cpu_id, vpid, vtid, procname, hostname)
}

#[derive(Debug)]
pub struct FullEvent {
    pub context: Context,
    pub time: Time,
    pub event: Event,
}

impl_from_for_enum! {
#[derive(Debug)]
pub enum Event {
    Ros2(ros2::Event),
    R2r(r2r::Event),
}
}

pub fn get_full_event(message: &BtMessageConst) -> Option<FullEvent> {
    let event = message.get_event();
    let context = context_from_event(&event);
    let time = time_from_message(message);

    let binding = event.get_class();
    let full_name = binding.get_name().unwrap();
    let provider = full_name.split(':').next().unwrap();

    let event = match provider {
        "ros2" => Event::Ros2(ros2::Event::from_event(&event).unwrap()),
        "r2r" => Event::R2r(r2r::Event::from_event(&event)?),
        _ => {
            return None;
        }
    };

    Some(FullEvent {
        context,
        time,
        event,
    })
}

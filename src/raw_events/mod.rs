use std::fmt::Debug;

use bt2_sys::{event::BtEventConst, message::BtMessageConst, trace::BtEnvironmentEntry};
use chrono::{DateTime, Local, TimeZone};

pub mod ros2;

pub mod r2r;

pub trait FromBtEvent: Sized {
    fn from_event(event: &BtEventConst) -> Option<Self>;
}

#[derive(Clone, PartialEq, Eq)]
pub struct Time {
    pub sec: i64,
    pub nsec: u32,
}

impl Time {
    pub fn new(sec: i64, nsec: u32) -> Self {
        Self { sec, nsec }
    }

    pub fn from_message(message: &BtMessageConst) -> Self {
        let clock_snapshot = message.get_default_clock_snapshot();
        let value_from_origin = clock_snapshot.get_value_from_origin().unwrap();
        Self::from_nanos(value_from_origin)
    }

    pub fn from_nanos(nanos: i64) -> Self {
        Self {
            sec: nanos.div_euclid(1_000_000_000),
            nsec: nanos.rem_euclid(1_000_000_000).try_into().unwrap(),
        }
    }

    pub fn as_datetime(&self) -> chrono::DateTime<chrono::Local> {
        Local.timestamp_nanos(self.sec * 1_000_000_000 + i64::from(self.nsec))
    }
}

impl Debug for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Time")
            .field(&self.as_datetime())
            .finish()
    }
    
}



#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContextId {
    vpid: u32,
    hostname: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    id: ContextId,
    cpu_id: u32,
    vtid: u32,
    procname: String,
}

impl Context {
    pub fn new(id: ContextId, cpu_id: u32, vtid: u32, procname: String) -> Self {
        Self {
            id,
            cpu_id,
            vtid,
            procname,
        }
    }

    pub fn from_event(event: &BtEventConst) -> Self {
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
            .unwrap()
        else {
            panic!("Missing hostname environment entry");
        };
        let hostname = hostname.get_value().to_string();

        let common_context = event.get_common_context_field().unwrap().into_struct();
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
        let id = ContextId { vpid, hostname };
        Self::new(id, cpu_id, vtid, procname)
    }
}

#[derive(Debug)]
pub struct FullEvent {
    pub context: Context,
    pub time: Time,
    pub event: Event,
}

#[derive(Debug)]
pub enum Event {
    Ros2(ros2::Event),
    R2r(r2r::Event),
}

pub fn get_full_event(message: &BtMessageConst) -> Option<FullEvent> {
    let event = message.get_event();
    let context = Context::from_event(&event);
    let time = Time::from_message(message);

    let binding = event.get_class();
    let full_name = binding.get_name().unwrap();
    let provider = full_name.split(':').next().unwrap();

    let event = match provider {
        "ros2" => Event::Ros2(ros2::Event::from_event(&event).unwrap()),
        "r2r" => Event::R2r(r2r::Event::from_event(&event)?),
        _ => { return None;},
    };

    Some(FullEvent {
        context,
        time,
        event,
    })
}

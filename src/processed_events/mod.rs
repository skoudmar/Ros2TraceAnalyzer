use crate::{events_common::{Context, Time}, impl_from_for_enum};

pub mod ros2;

pub mod r2r;

impl_from_for_enum! {
#[derive(Debug, Clone)]
pub enum Event {
    Ros2(ros2::Event),
    R2r(r2r::Event),
}
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Ros2(e) => write!(f, "ros2:{e}"),
            Event::R2r(e) => write!(f, "r2r:{e}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FullEvent {
    pub context: Context,
    pub time: Time,
    pub event: Event,
}

impl std::fmt::Display for FullEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {} {:?}", self.time, self.event, self.context)
    }
}

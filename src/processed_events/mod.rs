use std::sync::{Arc, Mutex};

use derive_more::derive::From;

use crate::events_common::{Context, Time};

pub mod ros2;

pub mod r2r;

type RefCount<T> = Arc<Mutex<T>>;

#[derive(Debug, Clone, From)]
pub enum Event {
    Ros2(ros2::Event),
    R2r(r2r::Event),
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ros2(e) => write!(f, "ros2:{e}"),
            Self::R2r(e) => write!(f, "r2r:{e}"),
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

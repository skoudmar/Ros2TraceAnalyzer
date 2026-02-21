use bt2_derive::TryFromBtFieldConst;
use bt2_sys::event::BtEventConst;
use derive_more::Debug;
use derive_more::derive::From;

use super::FromBtEvent;

#[derive(Debug, TryFromBtFieldConst, Clone)]
pub struct SpinStart {
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
    pub timeout_s: u64,
    pub timeout_ns: u32,
}

#[derive(Debug, TryFromBtFieldConst, Clone)]
pub struct SpinEnd {
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
}

#[derive(Debug, TryFromBtFieldConst, Clone)]
pub struct SpinWake {
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
}

#[derive(Debug, TryFromBtFieldConst, Clone)]
pub struct SpinTimeout {
    #[debug("{node_handle:#x}")]
    pub node_handle: u64,
}

#[derive(Debug, TryFromBtFieldConst, Clone)]
pub struct UpdateTime {
    #[debug("{subscriber:#x}")]
    pub subscriber: u64,
    pub time_s: i32,
    pub time_ns: u32,
}

#[derive(Debug, From, Clone)]
pub enum Event {
    SpinStart(SpinStart),
    SpinEnd(SpinEnd),
    SpinWake(SpinWake),
    SpinTimeout(SpinTimeout),
    UpdateTime(UpdateTime),
}
impl FromBtEvent for Event {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let event_class = event.get_class();
        let full_event_name = event_class.get_name().unwrap();
        let (provider_name, event_name) = full_event_name.split_once(':').unwrap();
        assert!(provider_name == "r2r");

        Some(match event_name {
            "spin_start" => SpinStart::from_event(event)?.into(),
            "spin_end" => SpinEnd::from_event(event)?.into(),
            "spin_wake" => SpinWake::from_event(event)?.into(),
            "spin_timeout" => SpinTimeout::from_event(event)?.into(),
            "update_time" => UpdateTime::from_event(event)?.into(),
            _ => return None,
        })
    }
}

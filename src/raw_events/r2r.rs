use bt2_derive::TryFromBtFieldConst;
use bt2_sys::event::BtEventConst;
use derive_more::derive::From;

use super::FromBtEvent;

#[derive(Debug, TryFromBtFieldConst)]
pub struct SpinStart {
    node_handle: u64,
    timeout_s: u64,
    timeout_ns: u64,
}

#[derive(Debug, From)]
pub enum Event {
    SpinStart(SpinStart),
}
impl FromBtEvent for Event {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        let event_class = event.get_class();
        let full_event_name = event_class.get_name().unwrap();
        let (provider_name, event_name) = full_event_name.split_once(':').unwrap();
        assert!(provider_name == "r2r");

        match event_name {
            "spin_start" => Some(Self::SpinStart(SpinStart::from_event(event)?)),
            _ => None,
        }
    }
}

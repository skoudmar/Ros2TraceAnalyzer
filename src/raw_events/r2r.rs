use bt2_sys::event::BtEventConst;

use super::FromBtEvent;

#[derive(Debug)]
pub enum Event {}
impl FromBtEvent for Event {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        // TODO: Implement this
        None
    }
}

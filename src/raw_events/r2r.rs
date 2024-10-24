use bt2_sys::event::BtEventConst;
use derive_more::derive::From;

use super::FromBtEvent;

#[derive(Debug, From)]
pub enum Event {}
impl FromBtEvent for Event {
    fn from_event(event: &BtEventConst) -> Option<Self> {
        // TODO: Implement this
        None
    }
}

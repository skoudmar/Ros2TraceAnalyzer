use std::{
    fmt::Debug, ops::{Deref, DerefMut}, ptr::NonNull
};

use crate::{clock_snapshot::BtClockSnapshotConst, event::BtEventConst, raw_bindings::{bt_message, bt_message_event_borrow_default_clock_snapshot_const, bt_message_event_borrow_event_const, bt_message_get_ref, bt_message_get_type, bt_message_put_ref, bt_message_type, bt_message_type_BT_MESSAGE_TYPE_DISCARDED_EVENTS, bt_message_type_BT_MESSAGE_TYPE_DISCARDED_PACKETS, bt_message_type_BT_MESSAGE_TYPE_EVENT, bt_message_type_BT_MESSAGE_TYPE_MESSAGE_ITERATOR_INACTIVITY, bt_message_type_BT_MESSAGE_TYPE_PACKET_BEGINNING, bt_message_type_BT_MESSAGE_TYPE_PACKET_END, bt_message_type_BT_MESSAGE_TYPE_STREAM_BEGINNING, bt_message_type_BT_MESSAGE_TYPE_STREAM_END}};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtMessageType {
    StreamBeginning,
    StreamEnd,
    Event,
    PacketBeginning,
    PacketEnd,
    DiscardedEvents,
    DiscardedPackets,
    MessageIteratorInactivity,
}

impl TryFrom<bt_message_type> for BtMessageType {
    type Error = ();
    
    fn try_from(value: bt_message_type) -> Result<Self, Self::Error> {
        Ok(match value {
            #![allow(non_upper_case_globals)]

            bt_message_type_BT_MESSAGE_TYPE_STREAM_BEGINNING => BtMessageType::StreamBeginning,
            bt_message_type_BT_MESSAGE_TYPE_EVENT => BtMessageType::Event,
            bt_message_type_BT_MESSAGE_TYPE_STREAM_END => BtMessageType::StreamEnd,
            bt_message_type_BT_MESSAGE_TYPE_PACKET_BEGINNING => BtMessageType::PacketBeginning,
            bt_message_type_BT_MESSAGE_TYPE_PACKET_END => BtMessageType::PacketEnd,
            bt_message_type_BT_MESSAGE_TYPE_DISCARDED_EVENTS => BtMessageType::DiscardedEvents,
            bt_message_type_BT_MESSAGE_TYPE_DISCARDED_PACKETS => BtMessageType::DiscardedPackets,
            bt_message_type_BT_MESSAGE_TYPE_MESSAGE_ITERATOR_INACTIVITY => {
                BtMessageType::MessageIteratorInactivity
            }
            _ => return Err(()),
        })
    }
}

#[repr(transparent)]
pub struct BtMessageConst(*const bt_message);

impl BtMessageConst {
    pub(crate) unsafe fn new_unchecked(message: *const bt_message) -> BtMessageConst {
        BtMessageConst(message)
    }

    pub fn get_type(&self) -> BtMessageType {
        debug_assert!(!self.0.is_null());
        unsafe { bt_message_get_type(self.0) }.try_into().unwrap()
    }

    pub fn get_event<'a>(&'a self) -> BtEventConst {
        debug_assert!(!self.0.is_null());
        assert!(self.get_type() == BtMessageType::Event);
        unsafe { BtEventConst::<'a>::new_unchecked(bt_message_event_borrow_event_const(self.0)) }
    }

    pub fn get_default_clock_snapshot(&self) -> BtClockSnapshotConst {
        debug_assert!(!self.0.is_null());
        unsafe { BtClockSnapshotConst::new_unchecked(bt_message_event_borrow_default_clock_snapshot_const(self.0)) }
    }

}

impl Clone for BtMessageConst {
    fn clone(&self) -> Self {
        debug_assert!(!self.0.is_null());
        unsafe {
            bt_message_get_ref(self.0);
        }

        unsafe { Self::new_unchecked(self.0) }
    }
}

impl Drop for BtMessageConst {
    fn drop(&mut self) {
        unsafe {
            bt_message_put_ref(self.0);
        }
    }
}

pub struct BtMessageArrayConst(NonNull<*const bt_message>, usize);
impl BtMessageArrayConst {
    pub(crate) unsafe fn new_unchecked(
        messages: *mut *const bt_message,
        count: u64,
    ) -> BtMessageArrayConst {
        BtMessageArrayConst(NonNull::new(messages).unwrap(), count as usize)
    }
}

impl Deref for BtMessageArrayConst {
    type Target = [BtMessageConst];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.0.cast().as_ptr(), self.1) }
    }
}

impl DerefMut for BtMessageArrayConst {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.0.cast().as_ptr(), self.1) }
    }
}

impl Default for BtMessageArrayConst {
    fn default() -> Self {
        Self(NonNull::dangling(), 0)
    }
}

impl Drop for BtMessageArrayConst {
    fn drop(&mut self) {
        for message in self.deref_mut() {
            unsafe {
                let message_ptr = (message as *mut BtMessageConst).cast::<*const bt_message>();
                std::ptr::drop_in_place(message);
                message_ptr.write(std::ptr::null());
            }
        }
    }
}

impl Debug for BtMessageArrayConst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter().map(|m| m.0)).finish()
    }
}

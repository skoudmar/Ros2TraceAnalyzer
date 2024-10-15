use std::ptr;

use crate::{
    error::{BtResult, IntoResult},
    message::BtMessageArrayConst,
    raw_bindings::{
        bt_message_iterator, bt_message_iterator_next,
    },
};

#[repr(transparent)]
pub struct BtMessageIterator(*mut bt_message_iterator);

impl BtMessageIterator {
    pub fn next(&mut self) -> BtResult<BtMessageArrayConst> {
        let mut count = 0;
        let mut messages = ptr::null_mut();
        unsafe { bt_message_iterator_next(self.0, &mut messages, &mut count) }
            .into_result()
            .map(|_| unsafe { BtMessageArrayConst::new_unchecked(messages, count) })
    }
}

impl From<*mut bt_message_iterator> for BtMessageIterator {
    fn from(iterator: *mut bt_message_iterator) -> Self {
        Self(iterator)
    }
}

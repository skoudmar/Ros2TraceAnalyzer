use std::marker::PhantomData;

use crate::{field::{BtFieldClassConst, BtFieldConst}, message::BtMessageConst, raw_bindings::{
    bt_event, bt_event_borrow_class_const, bt_event_borrow_common_context_field_const, bt_event_borrow_packet_const, bt_event_borrow_payload_field_const, bt_event_borrow_specific_context_field_const, bt_event_borrow_stream_const, bt_event_class, bt_event_class_borrow_payload_field_class_const, bt_event_class_get_id, bt_event_class_get_name
}, stream::{BtPacketConst, BtStreamConst}};

#[repr(transparent)]
pub struct BtEventConst<'a>(*const bt_event, PhantomData<&'a BtMessageConst>);

impl<'a> BtEventConst<'a> {
    pub(crate) unsafe fn new_unchecked(event: *const bt_event) -> Self {
        assert!(!event.is_null());
        BtEventConst(event, PhantomData)
    }

    #[inline]
    pub(crate) fn get_ptr(&self) -> *const bt_event {
        self.0
    }

    #[must_use] pub fn get_class(&self) -> BtEventClassConst {
        unsafe { BtEventClassConst::new_unchecked(bt_event_borrow_class_const(self.get_ptr())) }
    }

    #[must_use] pub fn get_payload(&self) -> Option<BtFieldConst> {
        let payload = unsafe { bt_event_borrow_payload_field_const(self.get_ptr()) };
        if payload.is_null() {
            return None;
        }

        Some(unsafe { BtFieldConst::new_unchecked(payload) })
    }

    #[must_use] pub fn get_common_context_field(&self) -> Option<BtFieldConst> {
        let common_context = unsafe { bt_event_borrow_common_context_field_const(self.get_ptr()) };
        if common_context.is_null() {
            return None;
        }

        Some(unsafe { BtFieldConst::new_unchecked(common_context) })
    }

    #[must_use] pub fn get_specific_context_field(&self) -> Option<BtFieldConst> {
        let specific_context = unsafe { bt_event_borrow_specific_context_field_const(self.get_ptr()) };
        if specific_context.is_null() {
            return None;
        }

        Some(unsafe { BtFieldConst::new_unchecked(specific_context) })
    }

    #[must_use] pub fn get_stream(&self) -> BtStreamConst {
        unsafe {
            BtStreamConst::new_unchecked(bt_event_borrow_stream_const(self.get_ptr()))
        }
    }

    #[must_use] pub fn get_packet(&self) -> BtPacketConst {
        assert!(self.get_stream().get_class().supports_packets(), "Event stream must support packets!");

        unsafe {BtPacketConst::new_unchecked(bt_event_borrow_packet_const(self.get_ptr()))}
    }
}

impl<'a> std::fmt::Debug for BtEventConst<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let class = self.get_class();
        let name = class.get_name().unwrap_or("Unknown");
        f.debug_struct(stringify!(BtEventConst))
            .field("name", &name)
            .field("payload", &self.get_payload())
            .field("common_context", &self.get_common_context_field())
            .field("specific_context", &self.get_specific_context_field())
            .finish()
    }
}

pub struct BtEventClassConst(*const bt_event_class);

impl BtEventClassConst {
    pub(crate) unsafe fn new_unchecked(event_class: *const bt_event_class) -> BtEventClassConst {
        BtEventClassConst(event_class)
    }

    #[inline(always)]
    pub(crate) fn get_ptr(&self) -> *const bt_event_class {
        self.0
    }

    pub fn get_id(&self) -> u64 {
        unsafe { bt_event_class_get_id(self.get_ptr()) }
    }

    pub fn get_name(&self) -> Option<&str> {
        Some(
            unsafe {
                let name = bt_event_class_get_name(self.get_ptr());
                if name.is_null() {
                    return None;
                }
                std::ffi::CStr::from_ptr(name)
            }
            .to_str()
            .expect("Failed to convert CStr to str"),
        )
    }

    pub fn get_payload_field_class(&self) -> BtFieldClassConst {
        unsafe { BtFieldClassConst::new_unchecked(bt_event_class_borrow_payload_field_class_const(self.get_ptr())) }
    }


}

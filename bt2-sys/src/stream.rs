use crate::field::BtFieldConst;
use crate::raw_bindings::{
    bt_packet, bt_packet_borrow_context_field_const, bt_packet_borrow_stream_const, bt_stream,
    bt_stream_borrow_class_const, bt_stream_borrow_trace_const, bt_stream_class,
    bt_stream_class_supports_packets,
};
use crate::trace::BtTraceConst;
use crate::utils::ConstNonNull;

#[repr(transparent)]
pub struct BtStreamConst(ConstNonNull<bt_stream>);

impl BtStreamConst {
    /// Create a new `BtStreamConst` from a raw pointer.
    ///
    /// # Safety
    /// The caller must ensure that the pointer is valid.
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_stream) -> Self {
        Self(ConstNonNull::new_unchecked(ptr))
    }

    #[inline]
    pub(crate) const fn get_ptr(&self) -> *const bt_stream {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn get_class(&self) -> BtStreamClassConst {
        unsafe { BtStreamClassConst::new_unchecked(bt_stream_borrow_class_const(self.get_ptr())) }
    }

    #[must_use]
    pub fn get_trace(&self) -> BtTraceConst {
        unsafe { BtTraceConst::new_unchecked(bt_stream_borrow_trace_const(self.get_ptr())) }
    }
}

#[repr(transparent)]
pub struct BtStreamClassConst(ConstNonNull<bt_stream_class>);

impl BtStreamClassConst {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_stream_class) -> Self {
        Self(ConstNonNull::new_unchecked(ptr))
    }

    #[inline]
    pub(crate) const fn get_ptr(&self) -> *const bt_stream_class {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn supports_packets(&self) -> bool {
        0 != unsafe { bt_stream_class_supports_packets(self.get_ptr()) }
    }
}

#[repr(transparent)]
pub struct BtPacketConst(ConstNonNull<bt_packet>);

impl BtPacketConst {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_packet) -> Self {
        Self(ConstNonNull::new_unchecked(ptr))
    }

    #[inline]
    pub(crate) const fn get_ptr(&self) -> *const bt_packet {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn get_context_field(&self) -> Option<BtFieldConst> {
        let field = unsafe { bt_packet_borrow_context_field_const(self.get_ptr()) };
        if field.is_null() {
            return None;
        }

        Some(unsafe { BtFieldConst::new_unchecked(field) })
    }

    #[must_use]
    pub fn get_stream(&self) -> BtStreamConst {
        unsafe { BtStreamConst::new_unchecked(bt_packet_borrow_stream_const(self.get_ptr())) }
    }
}

use std::ffi::CString;

use crate::{raw_bindings::{bt_trace, bt_trace_borrow_environment_entry_value_by_name_const}, value::{BtValueConst, BtValueSignedIntegerConst, BtValueStringConst, BtValueTypedConst}};

pub struct BtTraceConst(*const bt_trace);

pub enum BtEnvironmentEntry {
    Int(BtValueSignedIntegerConst),
    String(BtValueStringConst),
}

impl BtTraceConst {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_trace) -> Self {
        debug_assert!(!ptr.is_null());
        Self(ptr)
    }

    pub(crate) fn get_ptr(&self) -> *const bt_trace {
        self.0
    }

    pub fn get_environment_entry_by_name(&self, name: &str) -> Option<BtEnvironmentEntry> {
        let name = CString::new(name).unwrap();
        let value = unsafe { bt_trace_borrow_environment_entry_value_by_name_const(self.get_ptr(), name.as_ptr()) };
        if value.is_null() {
            return None;
        }

        let value = unsafe { BtValueConst::new_unchecked(value) };

        match value.get_type() {
            BtValueTypedConst::SignedInteger(value) => Some(BtEnvironmentEntry::Int(value)),
            BtValueTypedConst::String(value) => Some(BtEnvironmentEntry::String(value)),
            _ => unreachable!("Only signed integer and string environment entries are returned by the C API"),
        }
    }
}
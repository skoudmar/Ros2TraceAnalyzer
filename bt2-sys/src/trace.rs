use std::ffi::{CStr, CString};

use crate::raw_bindings::{bt_trace, bt_trace_borrow_environment_entry_value_by_name_const};
use crate::utils::ConstNonNull;
use crate::value::{
    BtValueConst, BtValueSignedIntegerConst, BtValueStringConst, BtValueTypedConst,
};

pub struct BtTraceConst(ConstNonNull<bt_trace>);

pub enum BtEnvironmentEntry {
    Int(BtValueSignedIntegerConst),
    String(BtValueStringConst),
}

impl BtTraceConst {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_trace) -> Self {
        Self(ConstNonNull::new_unchecked(ptr))
    }

    pub(crate) fn get_ptr(&self) -> *const bt_trace {
        self.0.as_ptr()
    }

    /// Get an environment entry by name.
    ///
    /// Returns `None` if the environment entry does not exist.
    ///
    /// # Panics
    ///
    /// Panics if the name contains a null byte.
    #[must_use]
    pub fn get_environment_entry_by_name(&self, name: &str) -> Option<BtEnvironmentEntry> {
        let name = CString::new(name).unwrap();
        self.get_environment_entry_by_name_cstr(name.as_ref())
    }

    /// Get an environment entry by name.
    ///
    /// Returns `None` if the environment entry does not exist.
    #[must_use]
    pub fn get_environment_entry_by_name_cstr(&self, name: &CStr) -> Option<BtEnvironmentEntry> {
        let value = unsafe {
            bt_trace_borrow_environment_entry_value_by_name_const(self.get_ptr(), name.as_ptr())
        };
        if value.is_null() {
            return None;
        }

        let value = unsafe { BtValueConst::new_unchecked(value) };

        match value.cast() {
            BtValueTypedConst::SignedInteger(value) => Some(BtEnvironmentEntry::Int(value)),
            BtValueTypedConst::String(value) => Some(BtEnvironmentEntry::String(value)),
            _ => unreachable!(
                "Only signed integer and string environment entries are returned by the C API"
            ),
        }
    }
}

use crate::raw_bindings::{
    bt_clock_class, bt_clock_class_origin_is_unix_epoch, bt_clock_snapshot,
    bt_clock_snapshot_borrow_clock_class_const, bt_clock_snapshot_get_ns_from_origin,
    bt_clock_snapshot_get_ns_from_origin_status, bt_clock_snapshot_get_value,
};
use crate::utils::ConstNonNull;

pub struct BtClockSnapshotConst(*const bt_clock_snapshot);

impl BtClockSnapshotConst {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_clock_snapshot) -> Self {
        debug_assert!(!ptr.is_null());
        Self(ptr)
    }

    pub(crate) const fn as_ptr(&self) -> *const bt_clock_snapshot {
        self.0
    }

    #[must_use]
    pub fn get_value(&self) -> u64 {
        unsafe { bt_clock_snapshot_get_value(self.as_ptr()) }
    }

    /// Returns the value of the clock snapshot from the origin of the clock or None if an overflow error occurred.
    #[must_use]
    pub fn get_value_from_origin(&self) -> Option<i64> {
        let mut ret_val = 0;
        let status = unsafe { bt_clock_snapshot_get_ns_from_origin(self.as_ptr(), &mut ret_val) };

        match status {
            bt_clock_snapshot_get_ns_from_origin_status::BT_CLOCK_SNAPSHOT_GET_NS_FROM_ORIGIN_STATUS_OK => Some(ret_val),
            bt_clock_snapshot_get_ns_from_origin_status::BT_CLOCK_SNAPSHOT_GET_NS_FROM_ORIGIN_STATUS_OVERFLOW_ERROR => None,
            status => unreachable!("Bug: Unknown bt_clock_snapshot_get_ns_from_origin_status: {}", status.0),
        }
    }

    #[must_use]
    pub fn get_class(&self) -> BtClockClassConst {
        unsafe {
            BtClockClassConst::new_unchecked(
                bt_clock_snapshot_borrow_clock_class_const(self.as_ptr())
                    .try_into()
                    .unwrap_unchecked(),
            )
        }
    }
}

pub struct BtClockClassConst(ConstNonNull<bt_clock_class>);

impl BtClockClassConst {
    pub(crate) const unsafe fn new_unchecked(ptr: ConstNonNull<bt_clock_class>) -> Self {
        Self(ptr)
    }

    pub(crate) const fn as_ptr(&self) -> *const bt_clock_class {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn origin_is_unix_epoch(&self) -> bool {
        0 != unsafe { bt_clock_class_origin_is_unix_epoch(self.as_ptr()) }
    }
}

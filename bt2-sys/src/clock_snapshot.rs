use crate::raw_bindings::{bt_clock_snapshot, bt_clock_snapshot_get_ns_from_origin, bt_clock_snapshot_get_ns_from_origin_status, bt_clock_snapshot_get_value};

pub struct BtClockSnapshotConst(*const bt_clock_snapshot);

impl BtClockSnapshotConst {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_clock_snapshot) -> Self {
        debug_assert!(!ptr.is_null());
        Self(ptr)
   }

    pub(crate) fn get_ptr(&self) -> *const bt_clock_snapshot {
        self.0
    }

    pub fn get_value(&self) -> u64 {
        unsafe { bt_clock_snapshot_get_value(self.0) }
    }

    /// Returns the value of the clock snapshot from the origin of the clock or None if an overflow error occurred.
    pub fn get_value_from_origin(&self) -> Option<i64> {
        let mut ret_val = 0;
        let status = unsafe { 
            bt_clock_snapshot_get_ns_from_origin(self.0, &mut ret_val)
        };

        match status {
            bt_clock_snapshot_get_ns_from_origin_status::BT_CLOCK_SNAPSHOT_GET_NS_FROM_ORIGIN_STATUS_OK => Some(ret_val),
            bt_clock_snapshot_get_ns_from_origin_status::BT_CLOCK_SNAPSHOT_GET_NS_FROM_ORIGIN_STATUS_OVERFLOW_ERROR => None,
        }
    }

}
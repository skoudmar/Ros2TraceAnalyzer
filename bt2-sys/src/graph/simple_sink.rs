use std::ffi::c_void;

use crate::raw_bindings::{
    bt_graph_simple_sink_component_consume_func_status,
    bt_graph_simple_sink_component_initialize_func_status, bt_message_iterator,
};

pub type InitializeFn =
    unsafe extern "C" fn(
        *mut bt_message_iterator,
        *mut c_void,
    ) -> bt_graph_simple_sink_component_initialize_func_status;

pub type ConsumeFn = unsafe extern "C" fn(
    *mut bt_message_iterator,
    *mut c_void,
) -> bt_graph_simple_sink_component_consume_func_status;

pub type FinalizeFn = unsafe extern "C" fn(*mut c_void);

pub struct SimpleSink {
    pub initialize_fn: Option<InitializeFn>,
    pub consume_fn: ConsumeFn,
    pub finalize_fn: Option<FinalizeFn>,
    pub user_data: *mut c_void,
}

use std::cell::RefCell;
use std::ffi::{c_void, CString};
use std::ptr::NonNull;
use std::rc::Rc;

use crate::error::{BtError, BtResult, IntoResult};
use crate::message::{BtMessageArrayConst, BtMessageConst};
use crate::raw_bindings::{
    bt_graph_simple_sink_component_consume_func_status, bt_message_iterator, destroy_trace_context,
    init_trace, next_events, sink, trace_context,
};
use crate::wrappers::BtMessageIterator;
use crate::{rethrow, throw};

pub(crate) enum MessageIteratorState {
    Running,
    Ended,
}

#[derive(Default)]
struct BatchMessageIteratorInner(RefCell<Option<BtMessageArrayConst>>);

pub(crate) struct BatchMessageIterator {
    trace_context: *mut trace_context,
    internal: Rc<BatchMessageIteratorInner>,
}

impl BatchMessageIterator {
    unsafe extern "C" fn consume(
        iterator: *mut bt_message_iterator,
        user_data: *mut c_void,
    ) -> bt_graph_simple_sink_component_consume_func_status {
        const STATUS_OK: bt_graph_simple_sink_component_consume_func_status =
            bt_graph_simple_sink_component_consume_func_status::BT_GRAPH_SIMPLE_SINK_COMPONENT_CONSUME_FUNC_STATUS_OK;
        const STATUS_END: bt_graph_simple_sink_component_consume_func_status =
            bt_graph_simple_sink_component_consume_func_status::BT_GRAPH_SIMPLE_SINK_COMPONENT_CONSUME_FUNC_STATUS_END;
        const STATUS_ERROR: bt_graph_simple_sink_component_consume_func_status =
            bt_graph_simple_sink_component_consume_func_status::BT_GRAPH_SIMPLE_SINK_COMPONENT_CONSUME_FUNC_STATUS_ERROR;

        let Some(this_ptr) = NonNull::new(user_data.cast::<BatchMessageIteratorInner>()) else {
            throw!("Failed to get MessageIterator from user_data");
            return STATUS_ERROR;
        };
        let this = this_ptr.as_ref();

        let mut iterator: BtMessageIterator = iterator.into();

        let Ok(mut internal) = this.0.try_borrow_mut() else {
            throw!("Failed to borrow internal state");
            return STATUS_ERROR;
        };
        if internal.is_some() {
            throw!("Internal state is not None");
            return STATUS_ERROR;
        }

        loop {
            let result = iterator.next();
            match result {
                Ok(messages) => {
                    *internal = Some(messages);

                    return STATUS_OK;
                }
                Err(BtError::Again(_)) => {
                    continue;
                }
                Err(BtError::End) => {
                    return STATUS_END;
                }
                Err(e) => {
                    rethrow!(e, "Failed to get next batch of messages");
                    return STATUS_ERROR;
                }
            }
        }
    }

    unsafe extern "C" fn finalize(user_data: *mut c_void) {
        if user_data.is_null() {
            eprintln!("User data is null in finalize function");
            std::process::abort();
        }
        let this_rc = Rc::from_raw(user_data.cast::<BatchMessageIteratorInner>());
        drop(this_rc);
    }
}

impl BatchMessageIterator {
    pub fn new(trace_path: &str) -> Self {
        let shared = Rc::new(BatchMessageIteratorInner::default());
        let shared_ptr = Rc::into_raw(shared.clone());
        let sink = sink {
            initialize_func: None,
            consume_func: Some(Self::consume),
            finalize_func: Some(Self::finalize),
            user_data: shared_ptr as *mut c_void,
        };

        let trace_path = CString::new(trace_path).unwrap();

        let trace_context = unsafe { init_trace(trace_path.as_ptr(), &sink) };
        debug_assert!(!trace_context.is_null());

        Self {
            trace_context,
            internal: shared,
        }
    }

    fn next_batch(&mut self) -> BtResult<MessageIteratorState> {
        let mut internal = self.internal.0.borrow_mut();
        let taken = internal.take();
        drop(taken);
        drop(internal);

        unsafe { next_events(self.trace_context) }.into_result()
    }
}

impl Drop for BatchMessageIterator {
    fn drop(&mut self) {
        let _ = self.internal.0.borrow_mut().take();

        unsafe {
            // FIXME: This might be dropped before the BtMessageArrayConst during panic
            destroy_trace_context(self.trace_context);
        }
    }
}

impl Iterator for BatchMessageIterator {
    type Item = BtMessageArrayConst;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_batch() {
            Ok(MessageIteratorState::Running) => {
                let mut internal = self.internal.0.borrow_mut();
                let messages = internal.take().unwrap();

                Some(messages)
            }
            Ok(MessageIteratorState::Ended) => None,
            Err(e) => {
                panic!("Failed to get next batch of messages: {e}");
            }
        }
    }
}

pub struct MessageIterator {
    batch_iterator: BatchMessageIterator,
    current_batch: Option<BtMessageArrayConst>,
    current_index: usize,
}

impl MessageIterator {
    #[must_use]
    pub fn new(trace_path: &str) -> Self {
        Self {
            batch_iterator: BatchMessageIterator::new(trace_path),
            current_batch: None,
            current_index: 0,
        }
    }
}

impl Iterator for MessageIterator {
    type Item = BtMessageConst;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            while self.current_batch.is_none()
                || self.current_index >= self.current_batch.as_ref().unwrap().len()
            {
                let _ = self.current_batch.take();
                match self.batch_iterator.next() {
                    Some(batch) => {
                        self.current_batch = Some(batch);
                        self.current_index = 0;
                    }
                    None => {
                        // No more batches - end of iterator
                        return None;
                    }
                }
            }

            let message = self.current_batch.as_ref().unwrap()[self.current_index].clone();
            self.current_index += 1;

            if message.is_none() {
                // Skip None messages
                // There should be no None messages in the batch
                continue;
            }

            return message;
        }
    }
}

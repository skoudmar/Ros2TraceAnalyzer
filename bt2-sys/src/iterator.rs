use std::cell::RefCell;
use std::ffi::{c_void, CStr, CString};
use std::ptr::NonNull;
use std::rc::Rc;

use thiserror::Error;

use crate::error::{BtError, BtResult, IntoResult, OutOfMemory};
use crate::graph::plugin::{BtPlugin, BtPluginLoadError};
use crate::graph::{AddComponentError, BtGraph, BtGraphBuilder, ConnectPortsError};
use crate::logging::LogLevel;
use crate::message::{BtMessageArrayConst, BtMessageConst};
use crate::raw_bindings::{
    bt_graph_run_once_status, bt_graph_simple_sink_component_consume_func_status,
    bt_message_iterator, sink,
};
use crate::value::{BtValueArray, BtValueMap, BtValueString};
use crate::wrappers::BtMessageIterator;
use crate::{rethrow, throw};

pub(crate) enum MessageIteratorState {
    Running,
    Ended,
}

#[derive(Default)]
struct BatchMessageIteratorInner(RefCell<Option<BtMessageArrayConst>>);

pub(crate) struct BatchMessageIterator {
    graph: BtGraph,
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

#[derive(Debug, Error)]
pub enum IteratorConstructionError {
    #[error(transparent)]
    OutOfMemory(#[from] OutOfMemory),

    #[error("Failed to load a plugin: {0}")]
    PluginError(#[from] BtPluginLoadError),

    #[error("Failed to load component: {0}")]
    ComponentLoadError(&'static str),

    #[error("Failed to add component to graph: {0}")]
    AddComponentError(#[from] AddComponentError),

    #[error("Failed to connect ports: {0}")]
    ConnectPortsError(#[from] ConnectPortsError),

    #[error(transparent)]
    Error(BtError),
}

impl BatchMessageIterator {
    pub fn new(trace_paths: &[&CStr]) -> Self {
        let shared = Rc::new(BatchMessageIteratorInner::default());
        let shared_ptr = Rc::into_raw(shared.clone());
        let sink = sink {
            initialize_func: None,
            consume_func: Some(Self::consume),
            finalize_func: Some(Self::finalize),
            user_data: shared_ptr as *mut c_void,
        };

        let graph_result = Self::construct_graph(trace_paths, sink, LogLevel::Info);

        match graph_result {
            Ok(graph) => Self {
                graph,
                internal: shared,
            },
            Err(e) => {
                let _ = unsafe { Rc::from_raw(shared_ptr) };
                panic!("Failed to construct trace processing graph: {e}");
            }
        }
    }

    fn construct_graph(
        trace_paths: &[&CStr],
        sink: sink,
        log_level: LogLevel,
    ) -> Result<BtGraph, IteratorConstructionError> {
        let mut graph = BtGraphBuilder::new()?;

        let ctf_plugin = BtPlugin::find_anywhere(c"ctf")?;
        let utils_plugin = BtPlugin::find_anywhere(c"utils")?;

        let source_class = ctf_plugin
            .borrow_source_component_class_by_name(c"fs")
            .ok_or(IteratorConstructionError::ComponentLoadError("fs"))?;
        let filter_class = utils_plugin
            .borrow_filter_component_class_by_name(c"muxer")
            .ok_or(IteratorConstructionError::ComponentLoadError("muxer"))?;

        let mut source_components = Vec::with_capacity(trace_paths.len());
        for (i, trace_path) in trace_paths.iter().enumerate() {
            let mut path = BtValueArray::new()?;
            path.push(&BtValueString::new_cstr(trace_path)?.into())?;
            let mut params = BtValueMap::new()?;
            params.insert_with_cstr_key(c"inputs", &path.into())?;

            let source_comp_name = format!("source_{i}");
            let source_comp_name = CString::new(source_comp_name).unwrap();
            let source = unsafe {
                graph.add_source_component_unchecked(
                    source_class,
                    &source_comp_name,
                    Some(params),
                    log_level,
                )
            }?;

            source_components.push(source);
        }

        let muxer = unsafe {
            graph.add_filter_component_unchecked(filter_class, c"muxer", None, log_level)
        }?;

        let mut muxer_input_ports_used = 0;
        for source in source_components {
            let port_count = source.get_output_port_count();

            for i in 0..port_count {
                let out_port = source.get_output_port(i);
                let in_port = muxer.get_input_port(muxer_input_ports_used);
                unsafe { graph.connect_ports_unchecked(out_port, in_port) }?;
                muxer_input_ports_used += 1;
            }
        }

        let sink = unsafe {
            graph.add_simple_sink_component_unchecked(
                c"rust_sink",
                sink.initialize_func,
                sink.consume_func,
                sink.finalize_func,
                sink.user_data,
            )
        }?;

        let out_port = muxer.get_output_port(0);
        let in_port = sink.get_input_port(0);
        unsafe { graph.connect_ports_unchecked(out_port, in_port) }?;

        Ok(graph.build())
    }

    fn next_batch(&mut self) -> BtResult<MessageIteratorState> {
        let mut internal = self.internal.0.borrow_mut();
        let taken = internal.take();
        drop(taken);
        drop(internal);

        let mut status = bt_graph_run_once_status::BT_GRAPH_RUN_ONCE_STATUS_AGAIN;

        while status == bt_graph_run_once_status::BT_GRAPH_RUN_ONCE_STATUS_AGAIN {
            unsafe {
                status = self.graph.run_once();
            }
        }

        status.into_result()
    }
}

impl Drop for BatchMessageIterator {
    fn drop(&mut self) {
        let _ = self.internal.0.borrow_mut().take();
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
    current_batch: Option<BtMessageArrayConst>,
    batch_iterator: BatchMessageIterator,
    current_index: usize,
}

impl MessageIterator {
    #[must_use]
    pub fn new(trace_paths: &[&CStr]) -> Self {
        Self {
            batch_iterator: BatchMessageIterator::new(trace_paths),
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

impl Drop for MessageIterator {
    fn drop(&mut self) {
        // Drop the current batch before any other resources
        let _ = self.current_batch.take();
    }
}

use std::ffi::CStr;
use std::ptr::{self, NonNull};

use component::{
    BtComponentClassFilterConst, BtComponentClassSinkConst, BtComponentClassSourceConst,
    BtComponentFilterConst, BtComponentSinkConst, BtComponentSourceConst,
};
use thiserror::Error;

use crate::error::{BtErrorWrapper, IntoResult, OutOfMemory};
use crate::logging::LogLevel;
use crate::raw_bindings::{
    bt_graph, bt_graph_add_component_status, bt_graph_add_filter_component,
    bt_graph_add_sink_component, bt_graph_add_source_component, bt_graph_create, bt_graph_put_ref,
    bt_graph_run, bt_graph_run_once, bt_graph_run_once_status, bt_graph_run_status,
};
use crate::value::BtValueMap;

pub mod component;
pub mod plugin;

#[repr(transparent)]
pub struct BtGraph(NonNull<bt_graph>);

impl BtGraph {
    pub fn builder() -> Result<BtGraphBuilder, OutOfMemory> {
        BtGraphBuilder::new()
    }

    const fn as_ptr(&self) -> *mut bt_graph {
        self.0.as_ptr()
    }

    pub unsafe fn run(&mut self) -> bt_graph_run_status {
        unsafe { bt_graph_run(self.as_ptr()) }
    }

    pub unsafe fn run_once(&mut self) -> bt_graph_run_once_status {
        unsafe { bt_graph_run_once(self.as_ptr()) }
    }
}

impl Drop for BtGraph {
    fn drop(&mut self) {
        unsafe {
            bt_graph_put_ref(self.as_ptr());
        }
    }
}

#[repr(transparent)]
pub struct BtGraphBuilder(BtGraph);

impl BtGraphBuilder {
    const MIP_VERSION: u64 = 0;

    /// Create a new graph builder.
    ///
    /// Use this builder to create a graph and then call [`build`](Self::build) to get the graph.
    ///
    /// # Errors
    /// If the memory allocation fails.
    pub fn new() -> Result<Self, OutOfMemory> {
        let graph = unsafe { bt_graph_create(Self::MIP_VERSION) };
        let graph = NonNull::new(graph).ok_or(OutOfMemory)?;
        Ok(Self(BtGraph(graph)))
    }

    const fn as_ptr(&self) -> *mut bt_graph {
        self.0.as_ptr()
    }

    /// Add a source component to the graph.
    ///
    /// # Safety
    /// The caller must ensure that the name is not used by another component in the graph.
    pub unsafe fn add_source_component_unchecked(
        &mut self,
        component_class: BtComponentClassSourceConst,
        name: &CStr,
        params: Option<BtValueMap>,
        log_level: LogLevel,
    ) -> Result<BtComponentSourceConst, AddComponentError> {
        let mut component_ptr = ptr::null();
        let params_ptr = params.as_ref().map_or(ptr::null(), |p| p.as_ptr());
        unsafe {
            // Safety: `params_ptr` null is allowed for emply params and `component_ptr` is a valid pointer.
            bt_graph_add_source_component(
                self.as_ptr(),
                component_class.as_ptr(),
                name.as_ptr(),
                params_ptr,
                log_level.into(),
                &mut component_ptr,
            )
        }
        .into_result()?;

        let component = unsafe { BtComponentSourceConst::new_unchecked(component_ptr) };

        Ok(component)
    }

    /// Add a filter component to the graph.
    ///
    /// # Safety
    /// The caller must ensure that the name is not used by another component in the graph.
    pub unsafe fn add_filter_component_unchecked(
        &mut self,
        component_class: BtComponentClassFilterConst,
        name: &CStr,
        params: Option<BtValueMap>,
        log_level: LogLevel,
    ) -> Result<BtComponentFilterConst, AddComponentError> {
        let mut component_ptr = ptr::null();
        let params_ptr = params.as_ref().map_or(ptr::null(), |p| p.as_ptr());
        unsafe {
            // Safety: `params_ptr` null is allowed for emply params and `component_ptr` is a valid pointer.
            bt_graph_add_filter_component(
                self.as_ptr(),
                component_class.as_ptr(),
                name.as_ptr(),
                params_ptr,
                log_level.into(),
                &mut component_ptr,
            )
        }
        .into_result()?;

        let component = unsafe { BtComponentFilterConst::new_unchecked(component_ptr) };

        Ok(component)
    }

    /// Add a sink component to the graph.
    ///
    /// # Safety
    /// The caller must ensure that the name is not used by another component in the graph.
    pub unsafe fn add_sink_component_unchecked(
        &mut self,
        component_class: BtComponentClassSinkConst,
        name: &CStr,
        params: Option<BtValueMap>,
        log_level: LogLevel,
    ) -> Result<BtComponentSinkConst, AddComponentError> {
        let mut component_ptr = ptr::null();
        let params_ptr = params.as_ref().map_or(ptr::null(), |p| p.as_ptr());
        unsafe {
            // Safety: `params_ptr` null is allowed for emply params and `component_ptr` is a valid pointer.
            bt_graph_add_sink_component(
                self.as_ptr(),
                component_class.as_ptr(),
                name.as_ptr(),
                params_ptr,
                log_level.into(),
                &mut component_ptr,
            )
        }
        .into_result()?;

        let component = unsafe { BtComponentSinkConst::new_unchecked(component_ptr) };

        Ok(component)
    }

    /// Build the graph.
    #[must_use]
    pub fn build(self) -> BtGraph {
        self.0
    }
}

#[derive(Debug, Error)]
pub enum AddComponentError {
    #[error(transparent)]
    OutOfMemory(#[from] OutOfMemory),
    #[error(transparent)]
    Error(#[from] BtErrorWrapper),
}

impl IntoResult<(), AddComponentError> for bt_graph_add_component_status {
    fn into_result(self) -> Result<(), AddComponentError> {
        match self {
            Self::BT_GRAPH_ADD_COMPONENT_STATUS_OK => Ok(()),
            Self::BT_GRAPH_ADD_COMPONENT_STATUS_MEMORY_ERROR => Err(OutOfMemory.into()),
            Self::BT_GRAPH_ADD_COMPONENT_STATUS_ERROR => Err(BtErrorWrapper::get()
                .expect("Error should be provided by the C API")
                .into()),
            _ => unreachable!("Unknown bt_graph_add_component_status value: {:?}", self),
        }
    }
}

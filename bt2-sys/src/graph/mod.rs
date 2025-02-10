use std::ffi::CStr;
use std::ptr::{self, NonNull};

use component::{
    BtComponentClassFilterConst, BtComponentClassSinkConst, BtComponentClassSourceConst,
    BtComponentFilterConst, BtComponentSinkConst, BtComponentSourceConst, BtPortInput,
    BtPortOutput,
};
use thiserror::Error;

use crate::error::{BtErrorWrapper, IntoResult, OutOfMemory};
use crate::logging::LogLevel;
use crate::raw_bindings::{
    bt_graph, bt_graph_add_component_status, bt_graph_add_filter_component,
    bt_graph_add_simple_sink_component, bt_graph_add_sink_component, bt_graph_add_source_component,
    bt_graph_connect_ports, bt_graph_connect_ports_status, bt_graph_create, bt_graph_put_ref,
    bt_graph_run, bt_graph_run_once, bt_graph_run_once_status, bt_graph_run_status,
};
use crate::value::BtValueMap;

pub mod component;
pub mod plugin;
pub mod simple_sink;

#[repr(transparent)]
pub struct BtGraph(NonNull<bt_graph>);

impl BtGraph {
    pub fn builder() -> Result<BtGraphBuilder, OutOfMemory> {
        BtGraphBuilder::new()
    }

    const fn as_ptr(&self) -> *mut bt_graph {
        self.0.as_ptr()
    }

    /// Run the graph to completion.
    pub unsafe fn run(&mut self) -> bt_graph_run_status {
        unsafe { bt_graph_run(self.as_ptr()) }
    }

    /// Run the graph once.
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

pub struct BtGraphBuilder {
    /// SAFETY: The graph must not be run in the builder.
    graph: BtGraph,
    failed_configuration: bool,
}

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
        Ok(Self {
            graph: BtGraph(graph),
            failed_configuration: false,
        })
    }

    /// Add a source component to the graph.
    ///
    /// # Panics
    /// If graph is faulty, i.e., previous configuration failed.
    ///
    /// # Safety
    /// The caller must ensure that the name is not used by another component in the graph
    /// and that the returned component's lifetime is shorter than the graph.
    pub unsafe fn add_source_component_unchecked<'a>(
        &mut self,
        component_class: BtComponentClassSourceConst,
        name: &CStr,
        params: Option<BtValueMap>,
        log_level: LogLevel,
    ) -> Result<BtComponentSourceConst<'a>, AddComponentError> {
        let mut guard = ConfigFailsafe::check(&mut self.failed_configuration);

        let mut component_ptr = ptr::null();
        let params_ptr = params.as_ref().map_or(ptr::null(), |p| p.as_ptr());
        unsafe {
            // Safety: `params_ptr` null is allowed for empty params and `component_ptr` is a valid pointer.
            bt_graph_add_source_component(
                self.graph.as_ptr(),
                component_class.as_ptr(),
                name.as_ptr(),
                params_ptr,
                log_level.into(),
                &mut component_ptr,
            )
        }
        .into_result()?;

        let component = unsafe { BtComponentSourceConst::new_unchecked(component_ptr) };
        guard.set_success();

        Ok(component)
    }

    /// Add a filter component to the graph.
    ///
    /// # Panics
    /// If graph is faulty, i.e., previous configuration failed.
    ///
    /// # Safety
    /// The caller must ensure that the name is not used by another component in the graph
    /// and that the returned component's lifetime is shorter than the graph.
    pub unsafe fn add_filter_component_unchecked<'a>(
        &mut self,
        component_class: BtComponentClassFilterConst,
        name: &CStr,
        params: Option<BtValueMap>,
        log_level: LogLevel,
    ) -> Result<BtComponentFilterConst<'a>, AddComponentError> {
        let mut guard = ConfigFailsafe::check(&mut self.failed_configuration);

        let mut component_ptr = ptr::null();
        let params_ptr = params.as_ref().map_or(ptr::null(), |p| p.as_ptr());
        unsafe {
            // Safety: `params_ptr` null is allowed for empty params and `component_ptr` is a valid pointer.
            bt_graph_add_filter_component(
                self.graph.as_ptr(),
                component_class.as_ptr(),
                name.as_ptr(),
                params_ptr,
                log_level.into(),
                &mut component_ptr,
            )
        }
        .into_result()?;

        let component = unsafe { BtComponentFilterConst::new_unchecked(component_ptr) };
        guard.set_success();

        Ok(component)
    }

    /// Add a sink component to the graph.
    ///
    /// # Panics
    /// If graph is faulty, i.e., previous configuration failed.
    ///
    /// # Safety
    /// The caller must ensure that the name is not used by another component in the graph
    /// and that the returned component's lifetime is shorter than the graph.
    pub unsafe fn add_sink_component_unchecked<'a>(
        &mut self,
        component_class: BtComponentClassSinkConst,
        name: &CStr,
        params: Option<BtValueMap>,
        log_level: LogLevel,
    ) -> Result<BtComponentSinkConst<'a>, AddComponentError> {
        let mut guard = ConfigFailsafe::check(&mut self.failed_configuration);

        let mut component_ptr = ptr::null();
        let params_ptr = params.as_ref().map_or(ptr::null(), |p| p.as_ptr());
        unsafe {
            // Safety: `params_ptr` null is allowed for empty params and `component_ptr` is a valid pointer.
            bt_graph_add_sink_component(
                self.graph.as_ptr(),
                component_class.as_ptr(),
                name.as_ptr(),
                params_ptr,
                log_level.into(),
                &mut component_ptr,
            )
        }
        .into_result()?;

        let component = unsafe { BtComponentSinkConst::new_unchecked(component_ptr) };
        guard.set_success();

        Ok(component)
    }

    /// Add a simple sink component to the graph.
    ///
    /// # Panics
    /// If graph is faulty, i.e., previous configuration failed.
    ///
    /// # Safety
    /// The caller must ensure that the name is not used by another component in the graph
    /// and that the returned component's lifetime is shorter than the graph.
    ///
    /// The `initialize_fn` and `finalize_fn` must be valid function pointers or `None`.
    /// The `consume_fn` must be a valid function pointer.
    /// The `user_data` must be a valid pointer or `null`.
    pub unsafe fn add_simple_sink_component_unchecked<'a>(
        &mut self,
        name: &CStr,
        initialize_fn: Option<simple_sink::InitializeFn>,
        consume_fn: simple_sink::ConsumeFn,
        finalize_fn: Option<simple_sink::FinalizeFn>,
        user_data: *mut std::ffi::c_void,
    ) -> Result<BtComponentSinkConst<'a>, AddComponentError> {
        let mut guard = ConfigFailsafe::check(&mut self.failed_configuration);

        let mut component_ptr = ptr::null();
        unsafe {
            bt_graph_add_simple_sink_component(
                self.graph.as_ptr(),
                name.as_ptr(),
                initialize_fn,
                Some(consume_fn),
                finalize_fn,
                user_data,
                &mut component_ptr,
            )
        }
        .into_result()?;

        let component = unsafe { BtComponentSinkConst::new_unchecked(component_ptr) };
        guard.set_success();

        Ok(component)
    }

    /// Connect two ports.
    ///
    /// # Panics
    /// If graph is faulty, i.e., previous configuration failed.
    ///
    /// # Safety
    /// The caller must ensure that the ports are not already connected.
    /// The ports must be from components of this graph.
    pub unsafe fn connect_ports_unchecked(
        &mut self,
        output: BtPortOutput,
        input: BtPortInput,
    ) -> Result<(), ConnectPortsError> {
        let mut guard = ConfigFailsafe::check(&mut self.failed_configuration);
        unsafe {
            bt_graph_connect_ports(
                self.graph.as_ptr(),
                output.as_ptr(),
                input.as_ptr(),
                ptr::null_mut(),
            )
        }
        .into_result()?;

        guard.set_success();
        Ok(())
    }

    /// Build the graph.
    ///
    /// # Panics
    /// If graph is faulty, i.e., previous configuration failed.
    #[must_use]
    pub fn build(self) -> BtGraph {
        assert!(
            !self.failed_configuration,
            "Cannot build a graph that has failed configuration"
        );
        self.graph
    }
}

struct ConfigFailsafe<'a> {
    success: bool,
    failed_configuration: &'a mut bool,
}

impl<'a> ConfigFailsafe<'a> {
    pub fn check(failed_configuration: &'a mut bool) -> Self {
        assert!(
            !*failed_configuration,
            "Cannot configure a graph that has already failed"
        );
        Self {
            success: false,
            failed_configuration,
        }
    }

    pub fn fail(&mut self) {
        *self.failed_configuration = true;
    }

    pub fn set_success(&mut self) {
        self.success = true;
    }
}

impl Drop for ConfigFailsafe<'_> {
    fn drop(&mut self) {
        if !self.success {
            self.fail();
        }
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

#[derive(Debug, Error)]
pub enum ConnectPortsError {
    #[error(transparent)]
    OutOfMemory(#[from] OutOfMemory),
    #[error(transparent)]
    Error(#[from] BtErrorWrapper),
}

impl IntoResult<(), ConnectPortsError> for bt_graph_connect_ports_status {
    fn into_result(self) -> Result<(), ConnectPortsError> {
        match self {
            Self::BT_GRAPH_CONNECT_PORTS_STATUS_OK => Ok(()),
            Self::BT_GRAPH_CONNECT_PORTS_STATUS_MEMORY_ERROR => Err(OutOfMemory.into()),
            Self::BT_GRAPH_CONNECT_PORTS_STATUS_ERROR => Err(BtErrorWrapper::get()
                .expect("Error should be provided by the C API")
                .into()),
            _ => unreachable!("Unknown bt_graph_add_component_status value: {:?}", self),
        }
    }
}

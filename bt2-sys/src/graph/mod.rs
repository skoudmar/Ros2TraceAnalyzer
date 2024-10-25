use std::ptr::NonNull;

use thiserror::Error;

use crate::error::{BtErrorWrapper, OutOfMemory};
use crate::raw_bindings::{bt_graph, bt_graph_create};

pub mod component;
pub mod plugin;

#[repr(transparent)]
pub struct BtGraph(NonNull<bt_graph>);

impl BtGraph {
    const MIP_VERSION: u64 = 0;

    pub fn new() -> Result<Self, OutOfMemory> {
        let graph = unsafe { bt_graph_create(Self::MIP_VERSION) };
        Ok(Self(NonNull::new(graph).ok_or(OutOfMemory)?))
    }

    fn as_ptr(&mut self) -> *mut bt_graph {
        self.0.as_ptr()
    }
}

#[derive(Debug, Error)]
pub enum AddComponentError {
    #[error(transparent)]
    OutOfMemory(#[from] OutOfMemory),
    #[error(transparent)]
    Error(BtErrorWrapper),
}

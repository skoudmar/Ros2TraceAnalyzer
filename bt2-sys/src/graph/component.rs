use std::ptr::NonNull;

use derive_more::derive::{Deref, DerefMut};

use crate::raw_bindings::{
    bt_component, bt_component_class, bt_component_class_type, bt_component_get_class_type,
    bt_component_get_ref, bt_component_put_ref,
};

#[repr(transparent)]
pub struct BtComponent(NonNull<bt_component>);

pub enum BtComponentCasted {
    Source(BtComponentSource),
    Filter(BtComponentFilter),
    Sink(BtComponentSink),
}

#[repr(transparent)]
#[derive(Clone, Deref, DerefMut)]
pub struct BtComponentSource(BtComponent);

#[repr(transparent)]
#[derive(Clone, Deref, DerefMut)]
pub struct BtComponentFilter(BtComponent);

#[repr(transparent)]
#[derive(Clone, Deref, DerefMut)]
pub struct BtComponentSink(BtComponent);

impl BtComponent {
    pub(crate) fn new(ptr: NonNull<bt_component>) -> Self {
        Self(ptr)
    }

    fn as_ptr(&self) -> *mut bt_component {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn cast(self) -> BtComponentCasted {
        let comp_type: bt_component_class_type =
            unsafe { bt_component_get_class_type(self.as_ptr()) };
        match comp_type {
            bt_component_class_type::BT_COMPONENT_CLASS_TYPE_SOURCE => {
                BtComponentCasted::Source(BtComponentSource(self))
            }
            bt_component_class_type::BT_COMPONENT_CLASS_TYPE_FILTER => {
                BtComponentCasted::Filter(BtComponentFilter(self))
            }
            bt_component_class_type::BT_COMPONENT_CLASS_TYPE_SINK => {
                BtComponentCasted::Sink(BtComponentSink(self))
            }
            _ => unreachable!("Unknown component type: {}", comp_type.0),
        }
    }
}

impl Clone for BtComponent {
    fn clone(&self) -> Self {
        unsafe {
            bt_component_get_ref(self.as_ptr());
        }

        Self::new(self.0)
    }
}

impl Drop for BtComponent {
    fn drop(&mut self) {
        unsafe {
            bt_component_put_ref(self.as_ptr());
        }
    }
}

pub struct BtComponentClassConst(*const bt_component_class);

impl BtComponentClassConst {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_component_class) -> Self {
        debug_assert!(!ptr.is_null());
        Self(ptr)
    }

    pub fn as_ptr(&self) -> *const bt_component_class {
        self.0
    }
}

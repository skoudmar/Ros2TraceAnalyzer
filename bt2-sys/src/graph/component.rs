use std::ffi::CStr;
use std::marker::PhantomData;
use std::ptr::NonNull;

use derive_more::derive::{Deref, DerefMut};

use crate::query::{
    BtQueryError, BtQueryExecutor, SupportInfoParams, SupportInfoResult, SupportInfoResultError,
};
use crate::raw_bindings::{
    bt_component, bt_component_class, bt_component_class_filter,
    bt_component_class_filter_as_component_class_const, bt_component_class_sink,
    bt_component_class_sink_as_component_class_const, bt_component_class_source,
    bt_component_class_source_as_component_class_const, bt_component_class_type,
    bt_component_get_class_type, bt_component_get_ref, bt_component_put_ref,
};
use crate::utils::ConstNonNull;
use crate::value::{BtValue, BtValueMap};

use super::plugin::BtPlugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BtComponentType {
    Source,
    Filter,
    Sink,
}

pub enum BtComponentCasted {
    Source(BtComponentSource),
    Filter(BtComponentFilter),
    Sink(BtComponentSink),
}

impl BtComponentCasted {
    #[must_use]
    pub fn as_type(&self) -> BtComponentType {
        match self {
            Self::Source(_) => BtComponentType::Source,
            Self::Filter(_) => BtComponentType::Filter,
            Self::Sink(_) => BtComponentType::Sink,
        }
    }
}

#[repr(transparent)]
pub struct BtComponent(NonNull<bt_component>);

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

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BtComponentClassConst<'a>(ConstNonNull<bt_component_class>, PhantomData<&'a BtPlugin>);

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BtComponentClassSourceConst<'a>(
    ConstNonNull<bt_component_class_source>,
    PhantomData<&'a BtPlugin>,
);

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BtComponentClassFilterConst<'a>(
    ConstNonNull<bt_component_class_filter>,
    PhantomData<&'a BtPlugin>,
);

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BtComponentClassSinkConst<'a>(
    ConstNonNull<bt_component_class_sink>,
    PhantomData<&'a BtPlugin>,
);

impl<'a> BtComponentClassConst<'a> {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_component_class) -> Self {
        Self(ConstNonNull::new_unchecked(ptr), PhantomData)
    }

    pub(crate) fn as_ptr(&self) -> *const bt_component_class {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn create_query(&self, object_name: &CStr, params: &BtValue) -> BtQueryExecutor {
        BtQueryExecutor::new(self, object_name, params)
    }

    /// Query the support information of this component class.
    ///
    /// Parameters depend on the component class.
    ///
    /// # Errors
    /// - If the component class does not support the `support-info` query, this function will return [`SupportInfoResultError::NotSupported`].
    /// - If the layout of the result of the query is not as expected, this function will return [`SupportInfoResultError::IncorrectLayout`].
    /// - If the result of the query contains a group that is not a string, this function will return [`SupportInfoResultError::GroupConversion`]
    /// - If the query fails, this function will return [`SupportInfoResultError::QueryError`].
    pub fn query_support_info(
        &self,
        params: SupportInfoParams,
    ) -> Result<SupportInfoResult, SupportInfoResultError> {
        let params: BtValueMap = params.try_into().map_err(BtQueryError::Memory)?;
        let mut query = self.create_query(c"babeltrace.support-info", &params);
        let result = match query.query() {
            Ok(result) => result,
            Err(BtQueryError::UnknownObject) => return Err(SupportInfoResultError::NotSupported),
            Err(err) => return Err(SupportInfoResultError::QueryError(err)),
        };
        SupportInfoResult::try_from(result)
    }
}

impl<'a> BtComponentClassSourceConst<'a> {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_component_class_source) -> Self {
        Self(ConstNonNull::new_unchecked(ptr), PhantomData)
    }

    pub(crate) fn as_ptr(&self) -> *const bt_component_class_source {
        self.0.as_ptr()
    }

    #[inline]
    pub(crate) fn as_ptr_upcast(&self) -> *const bt_component_class {
        unsafe { bt_component_class_source_as_component_class_const(self.as_ptr()) }
    }

    #[must_use]
    pub fn upcast(self) -> BtComponentClassConst<'a> {
        unsafe { BtComponentClassConst::new_unchecked(self.as_ptr_upcast()) }
    }
}

impl<'a> BtComponentClassFilterConst<'a> {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_component_class_filter) -> Self {
        Self(ConstNonNull::new_unchecked(ptr), PhantomData)
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *const bt_component_class_filter {
        self.0.as_ptr()
    }

    #[inline]
    pub(crate) fn as_ptr_upcast(&self) -> *const bt_component_class {
        unsafe { bt_component_class_filter_as_component_class_const(self.as_ptr()) }
    }

    #[must_use]
    pub fn upcast(self) -> BtComponentClassConst<'a> {
        unsafe { BtComponentClassConst::new_unchecked(self.as_ptr_upcast()) }
    }
}

impl<'a> BtComponentClassSinkConst<'a> {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_component_class_sink) -> Self {
        Self(ConstNonNull::new_unchecked(ptr), PhantomData)
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *const bt_component_class_sink {
        self.0.as_ptr()
    }

    #[inline]
    pub(crate) fn as_ptr_upcast(&self) -> *const bt_component_class {
        unsafe { bt_component_class_sink_as_component_class_const(self.as_ptr()) }
    }

    #[must_use]
    pub fn upcast(self) -> BtComponentClassConst<'a> {
        unsafe { BtComponentClassConst::new_unchecked(self.as_ptr_upcast()) }
    }
}

impl<'a> std::ops::Deref for BtComponentClassSourceConst<'a> {
    type Target = BtComponentClassConst<'a>;

    fn deref(&self) -> &Self::Target {
        assert_eq!(self.as_ptr_upcast().cast(), self.as_ptr());
        // Safety: The upcasted pointer is the same as the original pointer and the types have the same layout.
        unsafe { &*std::ptr::from_ref(self).cast::<Self::Target>() }
    }
}

impl<'a> std::ops::Deref for BtComponentClassFilterConst<'a> {
    type Target = BtComponentClassConst<'a>;

    fn deref(&self) -> &Self::Target {
        assert_eq!(self.as_ptr_upcast().cast(), self.as_ptr());
        // Safety: The upcasted pointer is the same as the original pointer and the types have the same layout.
        unsafe { &*std::ptr::from_ref(self).cast::<Self::Target>() }
    }
}

impl<'a> std::ops::Deref for BtComponentClassSinkConst<'a> {
    type Target = BtComponentClassConst<'a>;

    fn deref(&self) -> &Self::Target {
        assert_eq!(self.as_ptr_upcast().cast(), self.as_ptr());
        // Safety: The upcasted pointer is the same as the original pointer and the types have the same layout.
        unsafe { &*std::ptr::from_ref(self).cast::<Self::Target>() }
    }
}

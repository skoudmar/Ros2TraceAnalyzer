use std::ffi::CStr;
use std::marker::PhantomData;
use std::str::Utf8Error;

use derive_more::derive::From;

use crate::query::{
    BtQueryError, BtQueryExecutor, SupportInfoParams, SupportInfoResult, SupportInfoResultError,
};
use crate::raw_bindings::{
    bt_component, bt_component_class, bt_component_class_filter,
    bt_component_class_filter_as_component_class_const, bt_component_class_get_description,
    bt_component_class_get_help, bt_component_class_get_name, bt_component_class_get_type,
    bt_component_class_sink, bt_component_class_sink_as_component_class_const,
    bt_component_class_source, bt_component_class_source_as_component_class_const,
    bt_component_class_type, bt_component_filter, bt_component_get_class_type, bt_component_sink,
    bt_component_source,
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

impl From<bt_component_class_type> for BtComponentType {
    fn from(value: bt_component_class_type) -> Self {
        match value {
            bt_component_class_type::BT_COMPONENT_CLASS_TYPE_SOURCE => Self::Source,
            bt_component_class_type::BT_COMPONENT_CLASS_TYPE_FILTER => Self::Filter,
            bt_component_class_type::BT_COMPONENT_CLASS_TYPE_SINK => Self::Sink,
            _ => unreachable!("Unknown component type: {}", value.0),
        }
    }
}

#[derive(From)]
pub enum BtComponentCasted<'a> {
    Source(BtComponentSourceConst<'a>),
    Filter(BtComponentFilterConst<'a>),
    Sink(BtComponentSinkConst<'a>),
}

impl<'a> BtComponentCasted<'a> {
    #[must_use]
    pub const fn as_type(&self) -> BtComponentType {
        match self {
            Self::Source(_) => BtComponentType::Source,
            Self::Filter(_) => BtComponentType::Filter,
            Self::Sink(_) => BtComponentType::Sink,
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BtComponentConst<'a>(ConstNonNull<bt_component>, PhantomData<&'a bt_component>);

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BtComponentSourceConst<'a>(
    ConstNonNull<bt_component_source>,
    PhantomData<&'a bt_component_source>,
);

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BtComponentFilterConst<'a>(
    ConstNonNull<bt_component_filter>,
    PhantomData<&'a bt_component_filter>,
);

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BtComponentSinkConst<'a>(
    ConstNonNull<bt_component_sink>,
    PhantomData<&'a bt_component_sink>,
);

impl<'a> BtComponentConst<'a> {
    pub(crate) unsafe fn new(ptr: *const bt_component) -> Self {
        Self(ConstNonNull::new_unchecked(ptr), PhantomData)
    }

    const fn as_ptr(&self) -> *const bt_component {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn get_type(&self) -> BtComponentType {
        unsafe { bt_component_get_class_type(self.as_ptr()) }.into()
    }
}

impl<'a> BtComponentSourceConst<'a> {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_component_source) -> Self {
        Self(ConstNonNull::new_unchecked(ptr), PhantomData)
    }

    pub(crate) const fn as_ptr(&self) -> *const bt_component_source {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn upcast(self) -> BtComponentConst<'a> {
        unsafe { BtComponentConst::new(self.as_ptr().cast()) }
    }
}

impl<'a> BtComponentFilterConst<'a> {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_component_filter) -> Self {
        Self(ConstNonNull::new_unchecked(ptr), PhantomData)
    }

    pub(crate) const fn as_ptr(&self) -> *const bt_component_filter {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn upcast(self) -> BtComponentConst<'a> {
        unsafe { BtComponentConst::new(self.as_ptr().cast()) }
    }
}

impl<'a> BtComponentSinkConst<'a> {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_component_sink) -> Self {
        Self(ConstNonNull::new_unchecked(ptr), PhantomData)
    }

    pub(crate) const fn as_ptr(&self) -> *const bt_component_sink {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn upcast(self) -> BtComponentConst<'a> {
        unsafe { BtComponentConst::new(self.as_ptr().cast()) }
    }
}

#[derive(Debug, Clone, Copy, From)]
pub enum BtComponentClassCastedConst<'a> {
    Source(BtComponentClassSourceConst<'a>),
    Filter(BtComponentClassFilterConst<'a>),
    Sink(BtComponentClassSinkConst<'a>),
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

    /// Get the name of the component class.
    ///
    /// # Errors
    /// If the name is not valid UTF-8, this function will return an error.
    pub fn name(&self) -> Result<&str, Utf8Error> {
        unsafe {
            let ptr = bt_component_class_get_name(self.as_ptr());
            CStr::from_ptr(ptr).to_str()
        }
    }

    /// Get the description of the component class.
    ///
    /// # Errors
    /// If the description is not valid UTF-8, this function will return an error.
    #[must_use]
    pub fn description(&self) -> Option<Result<&str, Utf8Error>> {
        unsafe {
            let ptr = bt_component_class_get_description(self.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_str())
            }
        }
    }

    /// Get the help text of the component class.
    ///
    /// # Errors
    /// If the help text is not valid UTF-8, this function will return an error.
    #[must_use]
    pub fn help_text(&self) -> Option<Result<&str, Utf8Error>> {
        unsafe {
            let ptr = bt_component_class_get_help(self.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_str())
            }
        }
    }

    pub(crate) const fn as_ptr(&self) -> *const bt_component_class {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn get_type(&self) -> BtComponentType {
        unsafe { bt_component_class_get_type(self.as_ptr()) }.into()
    }

    #[must_use]
    pub fn cast(self) -> BtComponentClassCastedConst<'a> {
        match self.get_type() {
            BtComponentType::Source => BtComponentClassCastedConst::Source(unsafe {
                BtComponentClassSourceConst::new_unchecked(self.as_ptr().cast())
            }),
            BtComponentType::Filter => BtComponentClassCastedConst::Filter(unsafe {
                BtComponentClassFilterConst::new_unchecked(self.as_ptr().cast())
            }),
            BtComponentType::Sink => BtComponentClassCastedConst::Sink(unsafe {
                BtComponentClassSinkConst::new_unchecked(self.as_ptr().cast())
            }),
        }
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

    pub(crate) const fn as_ptr(&self) -> *const bt_component_class_source {
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
    pub(crate) const fn as_ptr(&self) -> *const bt_component_class_filter {
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
    pub(crate) const fn as_ptr(&self) -> *const bt_component_class_sink {
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

impl std::fmt::Debug for BtComponentClassConst<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BtComponentClassConst")
            .field("type", &self.get_type())
            .field("name", &self.name())
            .finish()
    }
}

impl std::fmt::Debug for BtComponentClassSourceConst<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BtComponentClassSourceConst")
            .field("name", &self.name())
            .finish()
    }
}

impl std::fmt::Debug for BtComponentClassFilterConst<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BtComponentClassFilterConst")
            .field("name", &self.name())
            .finish()
    }
}

impl std::fmt::Debug for BtComponentClassSinkConst<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BtComponentClassSinkConst")
            .field("name", &self.name())
            .finish()
    }
}

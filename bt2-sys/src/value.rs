use core::str;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ptr::NonNull;

use derive_more::derive::{Deref, DerefMut, Into};
use thiserror::Error;

use crate::error::OutOfMemory;
use crate::raw_bindings::{
    bt_value, bt_value_array_append_element, bt_value_array_append_element_status,
    bt_value_array_borrow_element_by_index_const, bt_value_array_create, bt_value_array_get_length,
    bt_value_array_is_empty, bt_value_bool_create_init, bt_value_bool_get, bt_value_bool_set,
    bt_value_get_ref, bt_value_get_type, bt_value_integer_signed_create_init,
    bt_value_integer_signed_get, bt_value_integer_signed_set,
    bt_value_integer_unsigned_create_init, bt_value_integer_unsigned_get,
    bt_value_integer_unsigned_set, bt_value_map_borrow_entry_value_const, bt_value_map_create,
    bt_value_map_insert_bool_entry, bt_value_map_insert_entry, bt_value_map_insert_entry_status,
    bt_value_map_insert_signed_integer_entry, bt_value_map_insert_string_entry,
    bt_value_map_insert_unsigned_integer_entry, bt_value_null, bt_value_put_ref,
    bt_value_real_create_init, bt_value_real_get, bt_value_real_set, bt_value_string_create_init,
    bt_value_string_get, bt_value_string_set, bt_value_string_set_status, bt_value_type,
};
use crate::utils::ConstNonNull;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BtValueType {
    Null,
    Bool,
    UnsignedInteger,
    SignedInteger,
    Real,
    String,
    Array,
    Map,
}

macro_rules! impl_try_from_using_cast {
    ($($($lt:lifetime),+ =>)? $enum:ident::$variant:ident, $from:ty, $to:ty) => {
        impl$(<$($lt),+>)? TryFrom<$from> for $to {
            type Error = IncorrectTypeError;

            fn try_from(value: $from) -> Result<Self, Self::Error> {
                match value.cast() {
                    $enum::$variant(value) => Ok(value),
                    got => Err(IncorrectTypeError {
                        expected: BtValueType::$variant,
                        got: got.get_type(),
                    }),
                }
            }
        }
    };
}

#[derive(Debug, Error, Copy, Clone)]
#[error("Incorrect BtValue type: expected {expected:?}, got {got:?}")]
pub struct IncorrectTypeError {
    expected: BtValueType,
    got: BtValueType,
}

pub enum BtValueTypedConst<'a> {
    Null(BtValueNullConst<'a>),
    Bool(BtValueBoolConst<'a>),
    UnsignedInteger(BtValueUnsignedIntegerConst<'a>),
    SignedInteger(BtValueSignedIntegerConst<'a>),
    Real(BtValueRealConst<'a>),
    String(BtValueStringConst<'a>),
    Array(BtValueArrayConst<'a>),
    Map(BtValueMapConst<'a>),
}

impl<'a> BtValueTypedConst<'a> {
    #[must_use]
    pub const fn get_type(&self) -> BtValueType {
        match self {
            Self::Null(_) => BtValueType::Null,
            Self::Bool(_) => BtValueType::Bool,
            Self::UnsignedInteger(_) => BtValueType::UnsignedInteger,
            Self::SignedInteger(_) => BtValueType::SignedInteger,
            Self::Real(_) => BtValueType::Real,
            Self::String(_) => BtValueType::String,
            Self::Array(_) => BtValueType::Array,
            Self::Map(_) => BtValueType::Map,
        }
    }
}

#[repr(transparent)]
pub struct BtValueConst<'a>(ConstNonNull<bt_value>, PhantomData<&'a bt_value>);

#[repr(transparent)]
#[derive(Deref, Into)]
pub struct BtValueNullConst<'a>(BtValueConst<'a>);

#[repr(transparent)]
#[derive(Deref, Into)]
pub struct BtValueBoolConst<'a>(BtValueConst<'a>);

#[repr(transparent)]
#[derive(Deref, Into)]
pub struct BtValueUnsignedIntegerConst<'a>(BtValueConst<'a>);

#[repr(transparent)]
#[derive(Deref, Into)]
pub struct BtValueSignedIntegerConst<'a>(BtValueConst<'a>);

#[repr(transparent)]
#[derive(Deref, Into)]
pub struct BtValueRealConst<'a>(BtValueConst<'a>);

#[repr(transparent)]
#[derive(Deref, Into)]
pub struct BtValueStringConst<'a>(BtValueConst<'a>);

#[repr(transparent)]
#[derive(Deref, Into)]
pub struct BtValueArrayConst<'a>(BtValueConst<'a>);

#[repr(transparent)]
#[derive(Deref, Into)]
pub struct BtValueMapConst<'a>(BtValueConst<'a>);

impl BtValueType {
    #[must_use]
    pub const fn from_raw(raw: bt_value_type) -> Option<Self> {
        match raw {
            bt_value_type::BT_VALUE_TYPE_NULL => Some(Self::Null),
            bt_value_type::BT_VALUE_TYPE_BOOL => Some(Self::Bool),
            bt_value_type::BT_VALUE_TYPE_UNSIGNED_INTEGER => Some(Self::UnsignedInteger),
            bt_value_type::BT_VALUE_TYPE_SIGNED_INTEGER => Some(Self::SignedInteger),
            bt_value_type::BT_VALUE_TYPE_REAL => Some(Self::Real),
            bt_value_type::BT_VALUE_TYPE_STRING => Some(Self::String),
            bt_value_type::BT_VALUE_TYPE_ARRAY => Some(Self::Array),
            bt_value_type::BT_VALUE_TYPE_MAP => Some(Self::Map),
            _ => None,
        }
    }
}

impl<'a> BtValueConst<'a> {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_value) -> Self {
        Self(ConstNonNull::new_unchecked(ptr), PhantomData)
    }

    pub(crate) const unsafe fn new(ptr: ConstNonNull<bt_value>) -> Self {
        Self(ptr, PhantomData)
    }

    #[inline]
    pub(crate) const fn as_ptr(&self) -> *const bt_value {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn cast(self) -> BtValueTypedConst<'a> {
        let typ = unsafe { bt_value_get_type(self.as_ptr()) };

        match BtValueType::from_raw(typ).expect("Unknown bt_value_type") {
            BtValueType::Null => BtValueTypedConst::Null(BtValueNullConst(self)),
            BtValueType::Bool => BtValueTypedConst::Bool(BtValueBoolConst(self)),
            BtValueType::UnsignedInteger => {
                BtValueTypedConst::UnsignedInteger(BtValueUnsignedIntegerConst(self))
            }
            BtValueType::SignedInteger => {
                BtValueTypedConst::SignedInteger(BtValueSignedIntegerConst(self))
            }
            BtValueType::Real => BtValueTypedConst::Real(BtValueRealConst(self)),
            BtValueType::String => BtValueTypedConst::String(BtValueStringConst(self)),
            BtValueType::Array => BtValueTypedConst::Array(BtValueArrayConst(self)),
            BtValueType::Map => BtValueTypedConst::Map(BtValueMapConst(self)),
        }
    }
}

impl_try_from_using_cast!('a => BtValueTypedConst::Null, BtValueConst<'a>, BtValueNullConst<'a>);

impl<'a> BtValueBoolConst<'a> {
    #[must_use]
    pub fn get(&self) -> bool {
        0 != unsafe { bt_value_bool_get(self.as_ptr()) }
    }
}

impl_try_from_using_cast!('a => BtValueTypedConst::Bool, BtValueConst<'a>, BtValueBoolConst<'a>);

impl<'a> BtValueUnsignedIntegerConst<'a> {
    #[must_use]
    pub fn get(&self) -> u64 {
        unsafe { bt_value_integer_unsigned_get(self.as_ptr()) }
    }
}

impl_try_from_using_cast!('a => BtValueTypedConst::UnsignedInteger, BtValueConst<'a>, BtValueUnsignedIntegerConst<'a>);

impl<'a> BtValueSignedIntegerConst<'a> {
    #[must_use]
    pub fn get(&self) -> i64 {
        unsafe { bt_value_integer_signed_get(self.as_ptr()) }
    }
}

impl_try_from_using_cast!('a => BtValueTypedConst::SignedInteger, BtValueConst<'a>, BtValueSignedIntegerConst<'a>);

impl<'a> BtValueStringConst<'a> {
    /// Get the contained string value.
    pub fn get(&self) -> Result<&str, str::Utf8Error> {
        unsafe {
            let ptr = bt_value_string_get(self.as_ptr());
            debug_assert!(!ptr.is_null());
            CStr::from_ptr(ptr).to_str()
        }
    }
}

impl_try_from_using_cast!('a => BtValueTypedConst::String, BtValueConst<'a>, BtValueStringConst<'a>);

impl<'a> BtValueRealConst<'a> {
    #[must_use]
    pub fn get(&self) -> f64 {
        unsafe { bt_value_real_get(self.as_ptr()) }
    }
}

impl_try_from_using_cast!('a => BtValueTypedConst::Real, BtValueConst<'a>, BtValueRealConst<'a>);

impl<'a> BtValueArrayConst<'a> {
    #[must_use]
    pub fn length(&self) -> u64 {
        unsafe { bt_value_array_get_length(self.as_ptr()) }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        unsafe { 0 != bt_value_array_is_empty(self.as_ptr()) }
    }

    #[must_use]
    pub fn get(&'a self, index: u64) -> BtValueConst<'a> {
        assert!(index < self.length());
        let ptr = unsafe { bt_value_array_borrow_element_by_index_const(self.as_ptr(), index) };
        unsafe { BtValueConst::new_unchecked(ptr) }
    }
}

impl_try_from_using_cast!('a => BtValueTypedConst::Array, BtValueConst<'a>, BtValueArrayConst<'a>);

impl<'a> BtValueMapConst<'a> {
    pub fn get(&self, key: &str) -> Option<BtValueConst<'a>> {
        let key = CString::new(key).expect("Key must not contain null byte");
        self.get_with_cstr_key(&key)
    }

    pub fn get_with_cstr_key(&self, key: &CStr) -> Option<BtValueConst<'a>> {
        let ptr = unsafe { bt_value_map_borrow_entry_value_const(self.as_ptr(), key.as_ptr()) };
        ConstNonNull::new(ptr).map(|ptr| unsafe { BtValueConst::new(ptr) })
    }
}

impl_try_from_using_cast!('a => BtValueTypedConst::Map, BtValueConst<'a>, BtValueMapConst<'a>);

#[repr(transparent)]
pub struct BtValue(NonNull<bt_value>);

pub enum BtValueTyped {
    Null(BtValueNull),
    Bool(BtValueBool),
    UnsignedInteger(BtValueUnsignedInteger),
    SignedInteger(BtValueSignedInteger),
    Real(BtValueReal),
    String(BtValueString),
    Array(BtValueArray),
    Map(BtValueMap),
}

impl BtValueTyped {
    #[must_use]
    pub const fn get_type(&self) -> BtValueType {
        match self {
            Self::Null(_) => BtValueType::Null,
            Self::Bool(_) => BtValueType::Bool,
            Self::UnsignedInteger(_) => BtValueType::UnsignedInteger,
            Self::SignedInteger(_) => BtValueType::SignedInteger,
            Self::Real(_) => BtValueType::Real,
            Self::String(_) => BtValueType::String,
            Self::Array(_) => BtValueType::Array,
            Self::Map(_) => BtValueType::Map,
        }
    }
}

#[repr(transparent)]
#[derive(Deref, DerefMut, Into)]
pub struct BtValueNull(BtValue);

#[repr(transparent)]
#[derive(Deref, DerefMut, Into)]
pub struct BtValueBool(BtValue);

#[repr(transparent)]
#[derive(Deref, DerefMut, Into)]
pub struct BtValueUnsignedInteger(BtValue);

#[repr(transparent)]
#[derive(Deref, DerefMut, Into)]
pub struct BtValueSignedInteger(BtValue);

#[repr(transparent)]
#[derive(Deref, DerefMut, Into)]
pub struct BtValueReal(BtValue);

#[repr(transparent)]
#[derive(Deref, DerefMut, Into)]
pub struct BtValueString(BtValue);

#[repr(transparent)]
#[derive(Deref, DerefMut, Into)]
pub struct BtValueArray(BtValue);

#[repr(transparent)]
#[derive(Deref, DerefMut, Into)]
pub struct BtValueMap(BtValue);

impl BtValue {
    const unsafe fn new_from_inner(ptr: NonNull<bt_value>) -> Self {
        Self(ptr)
    }

    pub(crate) const unsafe fn new_unchecked(ptr: *mut bt_value) -> Self {
        Self(NonNull::new_unchecked(ptr))
    }

    #[inline]
    pub(crate) const fn as_ptr(&self) -> *mut bt_value {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn cast(self) -> BtValueTyped {
        match self.get_type() {
            BtValueType::Null => BtValueTyped::Null(BtValueNull(self)),
            BtValueType::Bool => BtValueTyped::Bool(BtValueBool(self)),
            BtValueType::UnsignedInteger => {
                BtValueTyped::UnsignedInteger(BtValueUnsignedInteger(self))
            }
            BtValueType::SignedInteger => BtValueTyped::SignedInteger(BtValueSignedInteger(self)),
            BtValueType::Real => BtValueTyped::Real(BtValueReal(self)),
            BtValueType::String => BtValueTyped::String(BtValueString(self)),
            BtValueType::Array => BtValueTyped::Array(BtValueArray(self)),
            BtValueType::Map => BtValueTyped::Map(BtValueMap(self)),
        }
    }

    #[must_use]
    pub fn get_type(&self) -> BtValueType {
        BtValueType::from_raw(unsafe { bt_value_get_type(self.as_ptr()) })
            .expect("Unknown bt_value_type")
    }

    pub fn as_const(&self) -> BtValueConst<'_> {
        unsafe { BtValueConst::new_unchecked(self.as_ptr()) }
    }
}

impl Drop for BtValue {
    fn drop(&mut self) {
        unsafe { bt_value_put_ref(self.as_ptr()) }
    }
}

impl Default for BtValueNull {
    fn default() -> Self {
        unsafe {
            bt_value_get_ref(bt_value_null);
        };
        // Safety: bt_value_null is different from NULL
        Self(unsafe { BtValue::new_from_inner(NonNull::new_unchecked(bt_value_null)) })
    }
}

impl_try_from_using_cast!(BtValueTyped::Null, BtValue, BtValueNull);

impl BtValueBool {
    pub fn new(value: bool) -> Result<Self, OutOfMemory> {
        let ptr = unsafe { bt_value_bool_create_init(value.into()) };
        NonNull::new(ptr)
            .ok_or(OutOfMemory)
            .map(|ptr| unsafe { BtValue::new_from_inner(ptr) })
            .map(Self)
    }

    #[must_use]
    #[inline]
    pub fn get(&self) -> bool {
        0 != unsafe { bt_value_bool_get(self.as_ptr()) }
    }

    pub fn set(&mut self, value: bool) {
        unsafe { bt_value_bool_set(self.as_ptr(), value.into()) }
    }

    #[must_use]
    pub fn as_const(&self) -> BtValueBoolConst<'_> {
        BtValueBoolConst(self.0.as_const())
    }
}

impl_try_from_using_cast!(BtValueTyped::Bool, BtValue, BtValueBool);

impl BtValueUnsignedInteger {
    pub fn new(value: u64) -> Result<Self, OutOfMemory> {
        let ptr = unsafe { bt_value_integer_unsigned_create_init(value) };
        NonNull::new(ptr)
            .ok_or(OutOfMemory)
            .map(|ptr| unsafe { BtValue::new_from_inner(ptr) })
            .map(Self)
    }

    #[must_use]
    #[inline]
    pub fn get(&self) -> u64 {
        self.as_const().get()
    }

    pub fn set(&mut self, value: u64) {
        unsafe { bt_value_integer_unsigned_set(self.as_ptr(), value) }
    }

    #[must_use]
    pub fn as_const(&self) -> BtValueUnsignedIntegerConst<'_> {
        BtValueUnsignedIntegerConst(self.0.as_const())
    }
}

impl_try_from_using_cast!(
    BtValueTyped::UnsignedInteger,
    BtValue,
    BtValueUnsignedInteger
);

impl BtValueSignedInteger {
    pub fn new(value: i64) -> Result<Self, OutOfMemory> {
        let ptr = unsafe { bt_value_integer_signed_create_init(value) };
        NonNull::new(ptr)
            .ok_or(OutOfMemory)
            .map(|ptr| unsafe { BtValue::new_from_inner(ptr) })
            .map(Self)
    }

    #[must_use]
    #[inline]
    pub fn get(&self) -> i64 {
        self.as_const().get()
    }

    pub fn set(&mut self, value: i64) {
        unsafe { bt_value_integer_signed_set(self.as_ptr(), value) }
    }

    pub fn as_const(&self) -> BtValueSignedIntegerConst<'_> {
        BtValueSignedIntegerConst(self.0.as_const())
    }
}

impl_try_from_using_cast!(BtValueTyped::SignedInteger, BtValue, BtValueSignedInteger);

impl BtValueReal {
    pub fn new(value: f64) -> Result<Self, OutOfMemory> {
        let ptr = unsafe { bt_value_real_create_init(value) };
        NonNull::new(ptr)
            .ok_or(OutOfMemory)
            .map(|ptr| unsafe { BtValue::new_from_inner(ptr) })
            .map(Self)
    }

    #[must_use]
    #[inline]
    pub fn get(&self) -> f64 {
        self.as_const().get()
    }

    pub fn set(&mut self, value: f64) {
        unsafe { bt_value_real_set(self.as_ptr(), value) }
    }

    pub fn as_const(&self) -> BtValueRealConst<'_> {
        BtValueRealConst(self.0.as_const())
    }
}

impl_try_from_using_cast!(BtValueTyped::Real, BtValue, BtValueReal);

impl BtValueString {
    pub fn new(value: &str) -> Result<Self, OutOfMemory> {
        let cstring = CString::new(value).expect("Value must not contain null byte");
        Self::new_cstr(&cstring)
    }

    pub fn new_cstr(value: &CStr) -> Result<Self, OutOfMemory> {
        let ptr = unsafe { bt_value_string_create_init(value.as_ptr()) };
        NonNull::new(ptr)
            .ok_or(OutOfMemory)
            .map(|ptr| unsafe { BtValue::new_from_inner(ptr) })
            .map(Self)
    }

    pub fn get(&self) -> Result<&str, str::Utf8Error> {
        unsafe {
            let ptr = bt_value_string_get(self.as_ptr());
            debug_assert!(!ptr.is_null());
            CStr::from_ptr(ptr).to_str()
        }
    }

    pub fn set(&mut self, value: &str) -> Result<(), OutOfMemory> {
        let cstring = CString::new(value).expect("Value must not contain null byte");
        self.set_cstr(&cstring)
    }

    pub fn set_cstr(&mut self, value: &CStr) -> Result<(), OutOfMemory> {
        match unsafe { bt_value_string_set(self.as_ptr(), value.as_ptr()) } {
            bt_value_string_set_status::BT_VALUE_STRING_SET_STATUS_OK => Ok(()),
            bt_value_string_set_status::BT_VALUE_STRING_SET_STATUS_MEMORY_ERROR => Err(OutOfMemory),
            status => unreachable!("Bug: Unknown bt_value_string_set_status: {:?}", status.0),
        }
    }

    pub fn as_const(&self) -> BtValueStringConst<'_> {
        BtValueStringConst(self.0.as_const())
    }
}

impl_try_from_using_cast!(BtValueTyped::String, BtValue, BtValueString);

impl BtValueArray {
    pub fn new() -> Result<Self, OutOfMemory> {
        let ptr = unsafe { bt_value_array_create() };
        NonNull::new(ptr)
            .ok_or(OutOfMemory)
            .map(|ptr| unsafe { BtValue::new_from_inner(ptr) })
            .map(Self)
    }

    pub fn push(&mut self, value: &BtValue) -> Result<(), OutOfMemory> {
        unsafe { bt_value_array_append_element(self.as_ptr(), value.as_ptr()) }.into_result()
    }

    #[inline]
    pub fn length(&self) -> u64 {
        self.as_const().length()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.as_const().is_empty()
    }

    #[inline]
    pub fn as_const(&self) -> BtValueArrayConst<'_> {
        BtValueArrayConst(self.0.as_const())
    }

    #[must_use]
    pub fn get(&self, index: u64) -> BtValueConst<'_> {
        assert!(index < self.length());
        let ptr = unsafe { bt_value_array_borrow_element_by_index_const(self.as_ptr(), index) };
        unsafe { BtValueConst::new_unchecked(ptr) }
    }
}

impl_try_from_using_cast!(BtValueTyped::Array, BtValue, BtValueArray);

impl BtValueMap {
    pub fn new() -> Result<Self, OutOfMemory> {
        let ptr = unsafe { bt_value_map_create() };
        NonNull::new(ptr)
            .ok_or(OutOfMemory)
            .map(|ptr| unsafe { BtValue::new_from_inner(ptr) })
            .map(Self)
    }

    pub fn insert(&mut self, key: &str, value: &BtValue) -> Result<(), OutOfMemory> {
        let key = CString::new(key).expect("Key must not contain null byte");
        self.insert_with_cstr_key(&key, value)
    }

    pub fn insert_with_cstr_key(&mut self, key: &CStr, value: &BtValue) -> Result<(), OutOfMemory> {
        unsafe { bt_value_map_insert_entry(self.as_ptr(), key.as_ptr(), value.as_ptr()) }
            .into_result()
    }

    pub fn insert_null(&mut self, key: &str) -> Result<(), OutOfMemory> {
        let key = CString::new(key).expect("Key must not contain null byte");
        self.insert_null_with_cstr_key(&key)
    }

    pub fn insert_null_with_cstr_key(&mut self, key: &CStr) -> Result<(), OutOfMemory> {
        self.insert_with_cstr_key(key, &BtValueNull::default())
    }

    pub fn insert_bool(&mut self, key: &str, value: bool) -> Result<(), OutOfMemory> {
        let key = CString::new(key).expect("Key must not contain null byte");
        self.insert_bool_with_cstr_key(&key, value)
    }

    pub fn insert_bool_with_cstr_key(
        &mut self,
        key: &CStr,
        value: bool,
    ) -> Result<(), OutOfMemory> {
        unsafe { bt_value_map_insert_bool_entry(self.as_ptr(), key.as_ptr(), value.into()) }
            .into_result()
    }

    pub fn insert_unsigned_int(&mut self, key: &str, value: u64) -> Result<(), OutOfMemory> {
        let key = CString::new(key).expect("Key must not contain null byte");
        self.insert_unsigned_int_with_cstr_key(&key, value)
    }

    pub fn insert_unsigned_int_with_cstr_key(
        &mut self,
        key: &CStr,
        value: u64,
    ) -> Result<(), OutOfMemory> {
        unsafe { bt_value_map_insert_unsigned_integer_entry(self.as_ptr(), key.as_ptr(), value) }
            .into_result()
    }

    pub fn insert_int(&mut self, key: &str, value: i64) -> Result<(), OutOfMemory> {
        let key = CString::new(key).expect("Key must not contain null byte");
        self.insert_int_with_cstr_key(&key, value)
    }

    pub fn insert_int_with_cstr_key(&mut self, key: &CStr, value: i64) -> Result<(), OutOfMemory> {
        unsafe { bt_value_map_insert_signed_integer_entry(self.as_ptr(), key.as_ptr(), value) }
            .into_result()
    }

    pub fn insert_string(&mut self, key: &str, value: &str) -> Result<(), OutOfMemory> {
        let key = CString::new(key).expect("Key must not contain null byte");
        let value = CString::new(value).expect("Value must not contain null byte");
        self.insert_string_with_cstr_key_val(&key, &value)
    }

    pub fn insert_string_with_cstr_key_val(
        &mut self,
        key: &CStr,
        value: &CStr,
    ) -> Result<(), OutOfMemory> {
        unsafe { bt_value_map_insert_string_entry(self.as_ptr(), key.as_ptr(), value.as_ptr()) }
            .into_result()
    }

    pub fn as_const(&self) -> BtValueMapConst<'_> {
        BtValueMapConst(self.0.as_const())
    }

    pub fn get<'a>(&'a self, key: &str) -> Option<BtValueConst<'a>> {
        self.as_const().get(key)
    }

    pub fn get_with_cstr_key<'a>(&'a self, key: &CStr) -> Option<BtValueConst<'a>> {
        self.as_const().get_with_cstr_key(key)
    }
}

impl_try_from_using_cast!(BtValueTyped::Map, BtValue, BtValueMap);

trait InsertStatusIntoResult {
    fn into_result(self) -> Result<(), OutOfMemory>;
}

impl InsertStatusIntoResult for bt_value_array_append_element_status {
    fn into_result(self) -> Result<(), OutOfMemory> {
        match self {
            Self::BT_VALUE_ARRAY_APPEND_ELEMENT_STATUS_OK => Ok(()),
            Self::BT_VALUE_ARRAY_APPEND_ELEMENT_STATUS_MEMORY_ERROR => Err(OutOfMemory),
            status => unreachable!(
                "Bug: Unknown bt_value_array_append_element_status: {:?}",
                status.0
            ),
        }
    }
}

impl InsertStatusIntoResult for bt_value_map_insert_entry_status {
    fn into_result(self) -> Result<(), OutOfMemory> {
        match self {
            Self::BT_VALUE_MAP_INSERT_ENTRY_STATUS_OK => Ok(()),
            Self::BT_VALUE_MAP_INSERT_ENTRY_STATUS_MEMORY_ERROR => Err(OutOfMemory),
            status => unreachable!(
                "Bug: Unknown bt_value_map_insert_entry_status: {:?}",
                status.0
            ),
        }
    }
}

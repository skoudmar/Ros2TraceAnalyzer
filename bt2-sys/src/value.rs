use core::str;
use std::ffi::{CStr, CString};
use std::ptr::NonNull;

use derive_more::derive::{Deref, DerefMut};

use crate::error::OutOfMemory;
use crate::raw_bindings::{
    bt_value, bt_value_bool_create_init, bt_value_bool_get, bt_value_bool_set, bt_value_get_ref,
    bt_value_get_type, bt_value_integer_signed_get, bt_value_map_create,
    bt_value_map_insert_bool_entry, bt_value_map_insert_entry, bt_value_map_insert_entry_status,
    bt_value_map_insert_signed_integer_entry, bt_value_map_insert_string_entry,
    bt_value_map_insert_unsigned_integer_entry, bt_value_null, bt_value_put_ref,
    bt_value_string_create_init, bt_value_string_get, bt_value_string_set,
    bt_value_string_set_status, bt_value_type,
};
use crate::utils::ConstNonNull;

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

pub enum BtValueTypedConst {
    Null(BtValueNullConst),
    Bool(BtValueBoolConst),
    UnsignedInteger(BtValueUnsignedIntegerConst),
    SignedInteger(BtValueSignedIntegerConst),
    Real(BtValueRealConst),
    String(BtValueStringConst),
    Array(BtValueArrayConst),
    Map(BtValueMapConst),
}

#[repr(transparent)]
pub struct BtValueConst(ConstNonNull<bt_value>);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtValueNullConst(BtValueConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtValueBoolConst(BtValueConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtValueUnsignedIntegerConst(BtValueConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtValueSignedIntegerConst(BtValueConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtValueRealConst(BtValueConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtValueStringConst(BtValueConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtValueArrayConst(BtValueConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtValueMapConst(BtValueConst);

impl BtValueType {
    #[must_use]
    pub fn from_raw(raw: bt_value_type) -> Option<Self> {
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

impl BtValueConst {
    pub(crate) unsafe fn new_unchecked(ptr: *const bt_value) -> Self {
        Self(ConstNonNull::new_unchecked(ptr))
    }

    #[inline]
    pub(crate) fn get_ptr(&self) -> *const bt_value {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn cast(self) -> BtValueTypedConst {
        let typ = unsafe { bt_value_get_type(self.get_ptr()) };

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

impl BtValueSignedIntegerConst {
    #[must_use]
    pub fn get_value(&self) -> i64 {
        unsafe { bt_value_integer_signed_get(self.get_ptr()) }
    }
}

impl BtValueStringConst {
    /// Get the contained string value.
    ///
    /// # Panics
    ///
    /// Panics if value is not valid UTF-8.
    #[must_use]
    pub fn get_value(&self) -> &str {
        unsafe {
            let ptr = bt_value_string_get(self.get_ptr());
            debug_assert!(!ptr.is_null());
            CStr::from_ptr(ptr).to_str().expect("Invalid UTF-8")
        }
    }
}

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

#[repr(transparent)]
#[derive(Clone, Deref)]
pub struct BtValueNull(BtValue);

#[repr(transparent)]
#[derive(Clone, Deref)]
pub struct BtValueBool(BtValue);

#[repr(transparent)]
#[derive(Clone, Deref, DerefMut)]
pub struct BtValueUnsignedInteger(BtValue);

#[repr(transparent)]
#[derive(Clone, Deref, DerefMut)]
pub struct BtValueSignedInteger(BtValue);

#[repr(transparent)]
#[derive(Clone, Deref, DerefMut)]
pub struct BtValueReal(BtValue);

#[repr(transparent)]
#[derive(Clone, Deref, DerefMut)]
pub struct BtValueString(BtValue);

#[repr(transparent)]
#[derive(Clone, Deref, DerefMut)]
pub struct BtValueArray(BtValue);

#[repr(transparent)]
#[derive(Clone, Deref, DerefMut)]
pub struct BtValueMap(BtValue);

impl BtValue {
    fn new(ptr: NonNull<bt_value>) -> Self {
        Self(ptr)
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *mut bt_value {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn cast(self) -> BtValueTyped {
        let typ = unsafe { bt_value_get_type(self.as_ptr()) };

        match BtValueType::from_raw(typ).expect("Unknown bt_value_type") {
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
}

impl Clone for BtValue {
    fn clone(&self) -> Self {
        unsafe { bt_value_get_ref(self.as_ptr()) };
        Self::new(self.0)
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
        Self(unsafe { BtValue::new(NonNull::new_unchecked(bt_value_null)) })
    }
}

impl BtValueBool {
    pub fn new(value: bool) -> Result<Self, OutOfMemory> {
        let ptr = unsafe { bt_value_bool_create_init(value.into()) };
        NonNull::new(ptr)
            .ok_or(OutOfMemory)
            .map(BtValue::new)
            .map(Self)
    }

    pub fn get(&self) -> bool {
        0 != unsafe { bt_value_bool_get(self.as_ptr()) }
    }

    pub fn set(&mut self, value: bool) {
        unsafe { bt_value_bool_set(self.as_ptr(), value.into()) }
    }
}

impl BtValueString {
    pub fn new(value: &str) -> Result<Self, OutOfMemory> {
        let cstring = CString::new(value).expect("Value must not contain null byte");
        Self::new_cstr(&cstring)
    }

    pub fn new_cstr(value: &CStr) -> Result<Self, OutOfMemory> {
        let ptr = unsafe { bt_value_string_create_init(value.as_ptr()) };
        NonNull::new(ptr)
            .ok_or(OutOfMemory)
            .map(BtValue::new)
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
}

impl BtValueMap {
    pub fn new() -> Result<Self, OutOfMemory> {
        let ptr = unsafe { bt_value_map_create() };
        NonNull::new(ptr)
            .ok_or(OutOfMemory)
            .map(BtValue::new)
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
}

trait MapInsertStatusIntoResult {
    fn into_result(self) -> Result<(), OutOfMemory>;
}

impl MapInsertStatusIntoResult for bt_value_map_insert_entry_status {
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

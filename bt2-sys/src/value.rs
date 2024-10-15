use std::{ffi::CStr, ops::Deref};

use crate::{
    impl_deref,
    raw_bindings::{
        bt_value, bt_value_get_type, bt_value_integer_signed_get, bt_value_string_get,
        bt_value_type,
    },
};

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

#[non_exhaustive]
pub enum BtValueTypedConst {
    Null(BtValueNullConst),
    Bool(BtValueBoolConst),
    UnsignedInteger(BtValueUnsignedIntegerConst),
    SignedInteger(BtValueSignedIntegerConst),
    Real(BtValueRealConst),
    String(BtValueStringConst),
    // Array(Vec<BtValue>),
    // Map(BTreeMap<String, BtValue>),
}

#[repr(transparent)]
pub struct BtValueConst(*const bt_value);

#[repr(transparent)]
pub struct BtValueNullConst(BtValueConst);

#[repr(transparent)]
pub struct BtValueBoolConst(BtValueConst);

#[repr(transparent)]
pub struct BtValueUnsignedIntegerConst(BtValueConst);

#[repr(transparent)]
pub struct BtValueSignedIntegerConst(BtValueConst);

#[repr(transparent)]
pub struct BtValueRealConst(BtValueConst);

#[repr(transparent)]
pub struct BtValueStringConst(BtValueConst);

impl BtValueType {
    pub fn from_raw(raw: bt_value_type) -> Option<Self> {
        match raw {
            #![allow(non_upper_case_globals)]
            #![allow(non_snake_case)]
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
        debug_assert!(!ptr.is_null());
        Self(ptr)
    }

    #[inline(always)]
    pub(crate) fn get_ptr(&self) -> *const bt_value {
        debug_assert!(!self.0.is_null());
        self.0
    }

    pub fn get_type(self) -> BtValueTypedConst {
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
            BtValueType::Array => todo!(),
            BtValueType::Map => todo!(),
        }
    }
}

impl_deref!(BtValueConst; for BtValueNullConst, BtValueBoolConst, BtValueUnsignedIntegerConst, BtValueSignedIntegerConst, BtValueRealConst, BtValueStringConst);

impl BtValueSignedIntegerConst {
    pub fn get_value(&self) -> i64 {
        unsafe { bt_value_integer_signed_get(self.get_ptr()) }
    }
}

impl BtValueStringConst {
    pub fn get_value(&self) -> &str {
        unsafe {
            let ptr = bt_value_string_get(self.get_ptr());
            assert!(!ptr.is_null());
            CStr::from_ptr(ptr).to_str().unwrap()
        }
    }
}

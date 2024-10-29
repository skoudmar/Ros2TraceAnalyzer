use core::error;
use std::ffi::{CStr, CString};

use derive_more::derive::Deref;
use thiserror::Error;

use crate::raw_bindings::{
    bt_field, bt_field_array_borrow_element_field_by_index_const, bt_field_array_get_length,
    bt_field_bool_get_value, bt_field_borrow_class_const, bt_field_class,
    bt_field_class_structure_borrow_member_by_index_const,
    bt_field_class_structure_borrow_member_by_name_const,
    bt_field_class_structure_get_member_count, bt_field_class_structure_member,
    bt_field_class_structure_member_borrow_field_class_const,
    bt_field_class_structure_member_get_name, bt_field_class_type, bt_field_get_class_type,
    bt_field_integer_signed_get_value, bt_field_integer_unsigned_get_value,
    bt_field_string_get_length, bt_field_string_get_value,
    bt_field_structure_borrow_member_field_by_name_const,
};
use crate::utils::ConstNonNull;

#[repr(transparent)]
pub struct BtFieldConst(ConstNonNull<bt_field>);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtFieldBooleanConst(BtFieldConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtFieldUnsignedIntegerConst(BtFieldConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtFieldSignedIntegerConst(BtFieldConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtFieldStringConst(BtFieldConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtFieldArrayConst(BtFieldConst);

#[repr(transparent)]
#[derive(Deref)]
pub struct BtFieldStructureConst(BtFieldConst);

#[non_exhaustive]
pub enum BtFieldType {
    Boolean(BtFieldBooleanConst),
    UnsignedInteger(BtFieldUnsignedIntegerConst),
    SignedInteger(BtFieldSignedIntegerConst),
    String(BtFieldStringConst),
    Array(BtFieldArrayConst),
    Structure(BtFieldStructureConst),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum BtFieldClassType {
    Bool,
    BitArray,
    UnsignedInteger,
    SignedInteger,
    String,
    Array,
    Structure,
}

impl BtFieldConst {
    /// Create a new `BtFieldConst` from a raw pointer.
    ///
    /// # Safety
    /// The caller must ensure that the pointer is valid.
    pub(crate) unsafe fn new_unchecked(field: *const bt_field) -> Self {
        Self(ConstNonNull::new_unchecked(field))
    }

    fn as_ptr(&self) -> *const bt_field {
        self.0.as_ptr()
    }

    fn get_class(&self) -> BtFieldClassConst {
        unsafe { BtFieldClassConst::new_unchecked(bt_field_borrow_class_const(self.as_ptr())) }
    }

    #[must_use]
    pub fn get_class_type(&self) -> Option<BtFieldClassType> {
        let class = unsafe { bt_field_get_class_type(self.as_ptr()) };
        BtFieldClassType::from_field_class_type(class)
    }

    /// Cast the field into a specific field type.
    ///
    /// # Returns
    /// - `Ok(field)` if the field can be cast into the specified type.
    /// - `Err((field, class))` if the field cannot be cast into the specified type.
    ///     - `field` is the original field.
    ///     - `class` is the `class_type` of the field.
    ///
    /// # Errors
    /// The function will fail if the field type is not supported.
    ///
    /// # See also
    /// - [`Self::into_uint()`] to cast the field into an unsigned integer.
    /// - [`Self::into_int()`] to cast the field into a signed integer.
    /// - [`Self::into_string()`] to cast the field into a string.
    /// - [`Self::into_struct()`] to cast the field into a structure.
    /// - [`Self::into_array()`] to cast the field into an array.
    pub fn cast(self) -> Result<BtFieldType, (Self, bt_field_class_type)> {
        let class = unsafe { bt_field_get_class_type(self.as_ptr()) };
        let Some(matched) = BtFieldClassType::from_field_class_type(class) else {
            return Err((self, class));
        };

        Ok(match matched {
            BtFieldClassType::Bool => BtFieldType::Boolean(BtFieldBooleanConst(self)),
            BtFieldClassType::UnsignedInteger => {
                BtFieldType::UnsignedInteger(BtFieldUnsignedIntegerConst(self))
            }
            BtFieldClassType::SignedInteger => {
                BtFieldType::SignedInteger(BtFieldSignedIntegerConst(self))
            }
            BtFieldClassType::String => BtFieldType::String(BtFieldStringConst(self)),
            BtFieldClassType::Array => BtFieldType::Array(BtFieldArrayConst(self)),
            BtFieldClassType::Structure => BtFieldType::Structure(BtFieldStructureConst(self)),
            _ => unimplemented!(),
        })
    }

    pub fn into_bool(self) -> BtFieldBooleanConst {
        match self.cast() {
            Ok(BtFieldType::Boolean(inner)) => inner,
            Ok(t) => panic!("Expected boolean, got {:?}", t.get_class_type()),
            _ => panic!("Expected boolean, got unsupported type"),
        }
    }

    pub fn try_into_bool(self) -> Result<BtFieldBooleanConst, IncorrectTypeError> {
        match self.cast() {
            Ok(BtFieldType::Boolean(inner)) => Ok(inner),
            Ok(t) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::Bool,
                actual_type: Some(t.get_class_type()),
            }),
            Err(_) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::Bool,
                actual_type: None,
            }),
        }
    }

    /// Cast the field into an unsigned integer.
    ///
    /// # Panics
    /// If the field is not an unsigned integer.
    ///
    /// # See also
    /// - [`Self::cast()`] to also obtain the field type.
    #[must_use]
    pub fn into_uint(self) -> BtFieldUnsignedIntegerConst {
        match self.cast() {
            Ok(BtFieldType::UnsignedInteger(inner)) => inner,
            Ok(t) => panic!("Expected unsigned integer, got {:?}", t.get_class_type()),
            _ => panic!("Expected unsigned integer, got unsupported type"),
        }
    }

    pub fn try_into_uint(self) -> Result<BtFieldUnsignedIntegerConst, IncorrectTypeError> {
        match self.cast() {
            Ok(BtFieldType::UnsignedInteger(inner)) => Ok(inner),
            Ok(t) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::UnsignedInteger,
                actual_type: Some(t.get_class_type()),
            }),
            Err(_) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::UnsignedInteger,
                actual_type: None,
            }),
        }
    }

    /// Cast the field into a signed integer.
    ///
    /// # Panics
    /// If the field is not a signed integer.
    ///
    /// # See also
    /// - [`Self::cast()`] to also obtain the field type.
    #[must_use]
    pub fn into_int(self) -> BtFieldSignedIntegerConst {
        match self.cast() {
            Ok(BtFieldType::SignedInteger(inner)) => inner,
            Ok(t) => panic!("Expected signed integer, got {:?}", t.get_class_type()),
            _ => panic!("Expected signed integer, got unsupported type"),
        }
    }

    pub fn try_into_int(self) -> Result<BtFieldSignedIntegerConst, IncorrectTypeError> {
        match self.cast() {
            Ok(BtFieldType::SignedInteger(inner)) => Ok(inner),
            Ok(t) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::SignedInteger,
                actual_type: Some(t.get_class_type()),
            }),
            Err(_) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::SignedInteger,
                actual_type: None,
            }),
        }
    }

    /// Cast the field into a string.
    ///
    /// # Panics
    /// If the field is not a string.
    ///
    /// # See also
    /// - [`Self::cast()`] to also obtain the field type.
    #[must_use]
    pub fn into_string(self) -> BtFieldStringConst {
        match self.cast() {
            Ok(BtFieldType::String(inner)) => inner,
            Ok(t) => panic!("Expected string, got {:?}", t.get_class_type()),
            _ => panic!("Expected string, got unsupported type"),
        }
    }

    pub fn try_into_string(self) -> Result<BtFieldStringConst, IncorrectTypeError> {
        match self.cast() {
            Ok(BtFieldType::String(inner)) => Ok(inner),
            Ok(t) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::String,
                actual_type: Some(t.get_class_type()),
            }),
            Err(_) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::String,
                actual_type: None,
            }),
        }
    }

    /// Cast the field into a structure.
    ///
    /// # Panics
    /// If the field is not a structure.
    ///
    /// # See also
    /// - [`Self::cast()`] to also obtain the field type.
    #[must_use]
    pub fn into_struct(self) -> BtFieldStructureConst {
        match self.cast() {
            Ok(BtFieldType::Structure(inner)) => inner,
            Ok(t) => panic!("Expected structure, got {:?}", t.get_class_type()),
            _ => panic!("Expected structure, got unsupported type"),
        }
    }

    pub fn try_into_struct(self) -> Result<BtFieldStructureConst, IncorrectTypeError> {
        match self.cast() {
            Ok(BtFieldType::Structure(inner)) => Ok(inner),
            Ok(t) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::Structure,
                actual_type: Some(t.get_class_type()),
            }),
            Err(_) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::Structure,
                actual_type: None,
            }),
        }
    }

    /// Cast the field into an array.
    ///
    /// # Panics
    /// If the field is not an array.
    ///
    /// # See also
    /// - [`Self::cast()`] to also obtain the field type.
    #[must_use]
    pub fn into_array(self) -> BtFieldArrayConst {
        match self.cast() {
            Ok(BtFieldType::Array(inner)) => inner,
            Ok(t) => panic!("Expected array, got {:?}", t.get_class_type()),
            _ => panic!("Expected array, got unsupported type"),
        }
    }

    pub fn try_into_array(self) -> Result<BtFieldArrayConst, IncorrectTypeError> {
        match self.cast() {
            Ok(BtFieldType::Array(inner)) => Ok(inner),
            Ok(t) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::Array,
                actual_type: Some(t.get_class_type()),
            }),
            Err(_) => Err(IncorrectTypeError {
                requested_type: BtFieldClassType::Array,
                actual_type: None,
            }),
        }
    }

    pub(crate) unsafe fn clone_unchecked(&self) -> Self {
        unsafe { Self::new_unchecked(self.as_ptr()) }
    }
}

impl std::fmt::Debug for BtFieldConst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let casted = match unsafe { self.clone_unchecked() }.cast() {
            Ok(casted) => casted,
            Err((_, class)) => {
                return f
                    .debug_struct(stringify!(BtFieldConst))
                    .field("UnknownClass", &class)
                    .finish();
            }
        };

        match casted {
            BtFieldType::Boolean(inner) => inner.fmt(f),
            BtFieldType::UnsignedInteger(inner) => inner.fmt(f),
            BtFieldType::SignedInteger(inner) => inner.fmt(f),
            BtFieldType::String(inner) => inner.fmt(f),
            BtFieldType::Array(inner) => inner.fmt(f),
            BtFieldType::Structure(inner) => inner.fmt(f),
        }
    }
}

impl std::fmt::Display for BtFieldConst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let casted = match unsafe { self.clone_unchecked() }.cast() {
            Ok(casted) => casted,
            Err((_, class)) => {
                return write!(f, "Unknown field class: {}", class.0);
            }
        };

        match casted {
            BtFieldType::Boolean(inner) => inner.fmt(f),
            BtFieldType::UnsignedInteger(inner) => inner.fmt(f),
            BtFieldType::SignedInteger(inner) => inner.fmt(f),
            BtFieldType::String(inner) => inner.fmt(f),
            BtFieldType::Array(inner) => inner.fmt(f),
            BtFieldType::Structure(inner) => inner.fmt(f),
        }
    }
}

impl BtFieldType {
    #[must_use]
    pub fn get_class_type(&self) -> BtFieldClassType {
        match self {
            Self::Boolean(_) => BtFieldClassType::Bool,
            Self::UnsignedInteger(_) => BtFieldClassType::UnsignedInteger,
            Self::SignedInteger(_) => BtFieldClassType::SignedInteger,
            Self::String(_) => BtFieldClassType::String,
            Self::Array(_) => BtFieldClassType::Array,
            Self::Structure(_) => BtFieldClassType::Structure,
        }
    }
}

impl std::ops::Deref for BtFieldType {
    type Target = BtFieldConst;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Boolean(inner) => inner,
            Self::UnsignedInteger(inner) => inner,
            Self::SignedInteger(inner) => inner,
            Self::String(inner) => inner,
            Self::Array(inner) => inner,
            Self::Structure(inner) => inner,
        }
    }
}

macro_rules! impl_debug_and_display_for_scalar_field {
    ($type:ty) => {
        impl std::fmt::Debug for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($type))
                    .field(&self.get_value())
                    .finish()
            }
        }

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.get_value())
            }
        }
    };
}

impl BtFieldBooleanConst {
    /// Get the value of the boolean field.
    #[must_use]
    pub fn get_value(&self) -> bool {
        0 != unsafe { bt_field_bool_get_value(self.as_ptr()) }
    }
}

impl_debug_and_display_for_scalar_field!(BtFieldBooleanConst);

impl BtFieldUnsignedIntegerConst {
    /// Get the value of the unsigned integer field.
    #[must_use]
    pub fn get_value(&self) -> u64 {
        unsafe { bt_field_integer_unsigned_get_value(self.as_ptr()) }
    }
}

impl_debug_and_display_for_scalar_field!(BtFieldUnsignedIntegerConst);

impl BtFieldSignedIntegerConst {
    /// Get the value of the signed integer field.
    #[must_use]
    pub fn get_value(&self) -> i64 {
        unsafe { bt_field_integer_signed_get_value(self.as_ptr()) }
    }
}

impl_debug_and_display_for_scalar_field!(BtFieldSignedIntegerConst);

impl BtFieldStringConst {
    /// Get the length of the string field.
    #[must_use]
    pub fn get_length(&self) -> u64 {
        unsafe { bt_field_string_get_length(self.as_ptr()) }
    }

    /// Get the value of the string field.
    ///
    /// # Panics
    /// - If the string length cannot fit into `usize`.
    /// - If the string value cannot be converted to a `str`.
    #[must_use]
    pub fn get_value(&self) -> &str {
        let length = self
            .get_length()
            .try_into()
            .expect("String length is too large to convert to usize");
        unsafe {
            let ptr = bt_field_string_get_value(self.as_ptr());
            let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), length);
            std::str::from_utf8(bytes)
                .expect("BtFieldStringConst::get_value(): Failed to convert bytes to str")
        }
    }
}

impl_debug_and_display_for_scalar_field!(BtFieldStringConst);

impl BtFieldArrayConst {
    /// Get the length of the array.
    #[must_use]
    pub fn get_length(&self) -> u64 {
        unsafe { bt_field_array_get_length(self.as_ptr()) }
    }

    /// Get the value at the specified index.
    ///
    /// # Panics
    /// - If the index is out of bounds.
    #[must_use]
    pub fn get_value(&self, index: u64) -> BtFieldConst {
        assert!(index < self.get_length());

        unsafe {
            BtFieldConst::new_unchecked(bt_field_array_borrow_element_field_by_index_const(
                self.as_ptr(),
                index,
            ))
        }
    }

    /// Reads a static array of unsigned integers.
    ///
    /// # Panics
    /// - If the length of the array does not match `N`.
    /// - If the array contains a value that is not an unsigned integer.
    /// - If any of the elements cannot be converted to `T`.
    #[must_use]
    pub fn read_static_unsigned_array<const N: usize, T>(&self) -> [T; N]
    where
        T: TryFrom<u64>,
        <T as TryFrom<u64>>::Error: std::fmt::Debug,
    {
        assert!(N == self.get_length().try_into().unwrap());

        let mut i = 0;
        [0; N].map(|_| {
            let elem = self
                .get_value(i)
                .into_uint()
                .get_value()
                .try_into()
                .unwrap();

            i += 1;
            elem
        })
    }

    /// Read a static array of signed integers.
    ///
    /// # Panics
    /// - If the array length does not match the expected length `N`.
    /// - If the array contains a value that is not a signed integer.
    /// - If any of the elements cannot be converted to `T`.
    #[must_use]
    pub fn read_static_signed_array<const N: usize, T>(&self) -> [T; N]
    where
        T: TryFrom<i64>,
        <T as TryFrom<i64>>::Error: std::fmt::Debug,
    {
        assert!(N == self.get_length().try_into().unwrap());

        let mut i = 0;
        [0; N].map(|_| {
            let elem = self.get_value(i).into_int().get_value().try_into().unwrap();

            i += 1;
            elem
        })
    }
}

impl std::fmt::Debug for BtFieldArrayConst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries((0..self.get_length()).map(|i| self.get_value(i)))
            .finish()
    }
}

impl std::fmt::Display for BtFieldArrayConst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for i in 0..self.get_length() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", self.get_value(i))?;
        }
        write!(f, "]")
    }
}

impl BtFieldStructureConst {
    #[must_use]
    pub fn get_class(&self) -> BtFieldStructClassConst {
        BtFieldStructClassConst(self.0.get_class())
    }

    /// Get the number of fields in the structure.
    #[must_use]
    #[inline]
    pub fn get_field_count(&self) -> u64 {
        self.get_class().get_member_count()
    }

    /// Get the field with the specified name.
    ///
    /// The field `name` is converted to a [`CString`] before being passed to the C API.
    ///
    /// # Returns
    /// - `Some(field)` if the field exists.
    /// - `None` if the field does not exist.
    ///
    /// # Panics
    /// - If the field name cannot be converted to a [`CString`].
    ///
    /// # See also
    /// - [`Self::get_field_by_name_cstr()`] to avoid converting the `name` to a [`CString`].
    /// - [`Self::get_field_by_index()`] to get a field by its index.
    #[must_use]
    pub fn get_field_by_name(&self, name: &str) -> Option<BtFieldConst> {
        let name = CString::new(name).expect(
            "BtFieldStructureConst::get_field_by_name(): Failed to convert name to CString",
        );

        self.get_field_by_name_cstr(name.as_ref())
    }

    /// Get the field with the specified name.
    ///
    /// Note: This function is more efficient than [`Self::get_field_by_name()`] because it avoids converting the `name` to a [`CString`].
    ///
    /// # Returns
    /// - `Some(field)` if the field exists.
    /// - `None` if the field does not exist.
    ///
    /// # See also
    /// - [`Self::get_field_by_index()`] to get a field by its index.
    #[must_use]
    pub fn get_field_by_name_cstr(&self, name: &CStr) -> Option<BtFieldConst> {
        unsafe {
            let field =
                bt_field_structure_borrow_member_field_by_name_const(self.as_ptr(), name.as_ptr());
            if field.is_null() {
                return None;
            }

            Some(BtFieldConst::new_unchecked(field))
        }
    }

    /// Get the field at the specified index.
    ///
    /// # Panics
    /// - If the `index` is out of bounds.
    #[must_use]
    pub fn get_field_by_index(&self, index: u64) -> BtFieldConst {
        assert!(index < self.get_field_count());

        unsafe {
            BtFieldConst::new_unchecked(bt_field_array_borrow_element_field_by_index_const(
                self.as_ptr(),
                index,
            ))
        }
    }
}

impl std::fmt::Debug for BtFieldStructureConst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let class = self.get_class();
        let member_count = class.get_member_count();

        let mut s = f.debug_struct(stringify!(BtFieldStructureConst));
        for i in 0..member_count {
            let member_class = class.get_member_by_index(i);
            let member = self.get_field_by_index(i);
            s.field(member_class.get_name(), &member);
        }
        s.finish()
    }
}

impl std::fmt::Display for BtFieldStructureConst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let class = self.get_class();
        let member_count = class.get_member_count();

        write!(f, "{{")?;
        for i in 0..member_count {
            if i > 0 {
                write!(f, ", ")?;
            }

            let member_class = class.get_member_by_index(i);
            let member = self.get_field_by_index(i);
            write!(f, "{}: {}", member_class.get_name(), member)?;
        }
        write!(f, "}}")
    }
}

impl BtFieldClassType {
    #[must_use]
    pub(crate) fn from_field_class_type(class: bt_field_class_type) -> Option<Self> {
        Some(match class {
            bt_field_class_type::BT_FIELD_CLASS_TYPE_BOOL => Self::Bool,
            bt_field_class_type::BT_FIELD_CLASS_TYPE_UNSIGNED_INTEGER => Self::UnsignedInteger,
            bt_field_class_type::BT_FIELD_CLASS_TYPE_SIGNED_INTEGER => Self::SignedInteger,
            bt_field_class_type::BT_FIELD_CLASS_TYPE_STRING => Self::String,
            bt_field_class_type::BT_FIELD_CLASS_TYPE_STRUCTURE => Self::Structure,
            bt_field_class_type::BT_FIELD_CLASS_TYPE_STATIC_ARRAY
            | bt_field_class_type::BT_FIELD_CLASS_TYPE_DYNAMIC_ARRAY_WITHOUT_LENGTH_FIELD
            | bt_field_class_type::BT_FIELD_CLASS_TYPE_DYNAMIC_ARRAY_WITH_LENGTH_FIELD => {
                Self::Array
            }
            _ => {
                // TODO: Float
                // TODO: Enum
                // TODO: Bit array
                // TODO: Option
                // TODO: Variant
                return None;
            }
        })
    }
}

#[repr(transparent)]
pub struct BtFieldClassConst(ConstNonNull<bt_field_class>);

impl BtFieldClassConst {
    pub(crate) unsafe fn new_unchecked(field_class: *const bt_field_class) -> Self {
        Self(ConstNonNull::new_unchecked(field_class))
    }

    fn as_ptr(&self) -> *const bt_field_class {
        self.0.as_ptr()
    }
}

#[repr(transparent)]
#[derive(Deref)]
pub struct BtFieldStructClassConst(BtFieldClassConst);

#[repr(transparent)]
pub struct BtFieldStructMemberClassConst(ConstNonNull<bt_field_class_structure_member>);

impl BtFieldStructClassConst {
    #[must_use]
    pub fn get_member_count(&self) -> u64 {
        unsafe { bt_field_class_structure_get_member_count(self.as_ptr()) }
    }

    /// Get the member with the specified `name`.
    ///
    /// The field `name` is converted to a [`CString`] before being passed to the C API.
    ///
    /// # Returns
    /// - `Some(field)` if the field exists.
    /// - `None` if the field does not exist.
    ///
    /// # Panics
    /// - If the field name cannot be converted to a [`CString`].
    ///
    /// # See also
    /// - [`Self::get_member_by_name_cstr()`] to avoid converting the `name` to a [`CString`].
    /// - [`Self::get_member_by_index()`] to get a field by its index.
    #[must_use]
    pub fn get_member_by_name(&self, name: &str) -> Option<BtFieldStructMemberClassConst> {
        let name = CString::new(name).expect(
            "BtFieldStructClassConst::get_field_by_name(): Failed to convert &str to CString",
        );

        self.get_member_by_name_cstr(name.as_ref())
    }

    /// Get the member with the specified `name` as [`CStr`].
    ///
    /// Note: This function is more efficient than [`Self::get_member_by_name()`] because it avoids converting the `name` to a [`CString`].
    ///
    /// # Returns
    /// - `Some(field)` if the field exists.
    /// - `None` if the field does not exist.
    ///
    /// # See also
    /// - [`Self::get_member_by_name()`] to convert the `name` to a [`CString`] before passing it to the C API.
    /// - [`Self::get_member_by_index()`] to get a field by its index.
    #[must_use]
    pub fn get_member_by_name_cstr(&self, name: &CStr) -> Option<BtFieldStructMemberClassConst> {
        unsafe {
            let field =
                bt_field_class_structure_borrow_member_by_name_const(self.as_ptr(), name.as_ptr());
            if field.is_null() {
                return None;
            }

            Some(BtFieldStructMemberClassConst::new_unchecked(field))
        }
    }

    /// Get the member at the specified `index`.
    ///
    /// # Panics
    /// - If the `index` is out of bounds.
    ///
    /// # See also
    /// - [`Self::get_member_count()`] to get the number of members in the structure.
    #[must_use]
    pub fn get_member_by_index(&self, index: u64) -> BtFieldStructMemberClassConst {
        assert!(index < self.get_member_count());

        unsafe {
            BtFieldStructMemberClassConst::new_unchecked(
                bt_field_class_structure_borrow_member_by_index_const(self.as_ptr(), index),
            )
        }
    }
}

impl BtFieldStructMemberClassConst {
    pub(crate) unsafe fn new_unchecked(
        field_class: *const bt_field_class_structure_member,
    ) -> Self {
        Self(ConstNonNull::new_unchecked(field_class))
    }

    fn as_ptr(&self) -> *const bt_field_class_structure_member {
        self.0.as_ptr()
    }

    /// Get the name of the structure member.
    ///
    /// # Panics
    /// - If the name cannot be converted to a `str`.
    #[must_use]
    pub fn get_name(&self) -> &str {
        unsafe {
            let name = bt_field_class_structure_member_get_name(self.as_ptr());
            std::ffi::CStr::from_ptr(name)
        }
        .to_str()
        .expect("Failed to convert CStr to str")
    }

    #[must_use]
    pub fn get_class(&self) -> BtFieldClassConst {
        unsafe {
            BtFieldClassConst::new_unchecked(
                bt_field_class_structure_member_borrow_field_class_const(self.as_ptr()),
            )
        }
    }
}

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Cannot convert returned value to string. The value is not valid UTF-8!")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("Cannot convert value. Not a lossless conversion!")]
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    StructConversionError(#[from] StructConversionError),
    #[error(transparent)]
    IncorrectTypeError(#[from] IncorrectTypeError),
}


#[derive(Debug, Error)]
#[error("Cannot convert value. BtFieldConst has different class type: expected {requested_type:?}, got {actual_type:?}")]
pub struct IncorrectTypeError {
    requested_type: BtFieldClassType,
    actual_type: Option<BtFieldClassType>,
}

#[derive(Debug, Error)]
pub enum StructConversionError {
    #[error("Struct field not found: {0}")]
    FieldNotFound(&'static str),
    #[error("Struct field {1}: {0}")]
    FieldConversonError(#[source] Box<ConversionError>, &'static str),
}

use std::ops::Deref;

use crate::raw_bindings::{
    bt_field, bt_field_array_borrow_element_field_by_index_const, bt_field_array_get_length,
    bt_field_borrow_class_const, bt_field_class,
    bt_field_class_structure_borrow_member_by_index_const,
    bt_field_class_structure_borrow_member_by_name_const,
    bt_field_class_structure_get_member_count, bt_field_class_structure_member,
    bt_field_class_structure_member_borrow_field_class_const,
    bt_field_class_structure_member_get_name, bt_field_class_type_BT_FIELD_CLASS_TYPE_BOOL,
    bt_field_class_type_BT_FIELD_CLASS_TYPE_DYNAMIC_ARRAY_WITHOUT_LENGTH_FIELD,
    bt_field_class_type_BT_FIELD_CLASS_TYPE_DYNAMIC_ARRAY_WITH_LENGTH_FIELD,
    bt_field_class_type_BT_FIELD_CLASS_TYPE_SIGNED_INTEGER,
    bt_field_class_type_BT_FIELD_CLASS_TYPE_STATIC_ARRAY,
    bt_field_class_type_BT_FIELD_CLASS_TYPE_STRING,
    bt_field_class_type_BT_FIELD_CLASS_TYPE_STRUCTURE,
    bt_field_class_type_BT_FIELD_CLASS_TYPE_UNSIGNED_INTEGER, bt_field_get_class_type,
    bt_field_integer_unsigned_get_value, bt_field_string_get_length, bt_field_string_get_value,
    bt_field_structure_borrow_member_field_by_name_const,
};

#[repr(transparent)]
pub struct BtFieldConst(*const bt_field);

#[repr(transparent)]
pub struct BtFieldUnsignedIntegerConst(BtFieldConst);

#[repr(transparent)]
pub struct BtFieldSignedIntegerConst(BtFieldConst);

#[repr(transparent)]
pub struct BtFieldStringConst(BtFieldConst);

#[repr(transparent)]
pub struct BtFieldArrayConst(BtFieldConst);

#[repr(transparent)]
pub struct BtFieldStructureConst(BtFieldConst);

#[non_exhaustive]
pub enum BtFieldType {
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
    pub(crate) unsafe fn new_unchecked(field: *const bt_field) -> BtFieldConst {
        BtFieldConst(field)
    }

    fn as_ptr(&self) -> *const bt_field {
        self.0
    }

    fn get_class(&self) -> BtFieldClassConst {
        unsafe { BtFieldClassConst::new_unchecked(bt_field_borrow_class_const(self.as_ptr())) }
    }

    #[must_use]
    pub fn get_class_type(&self) -> Option<BtFieldClassType> {
        let class = unsafe { bt_field_get_class_type(self.as_ptr()) };
        BtFieldClassType::from_field_class_type(class)
    }

    pub fn cast(self) -> Result<BtFieldType, (Self, bt_field_class_type)> {
        let class = unsafe { bt_field_get_class_type(self.as_ptr()) };
        let Some(matched) = BtFieldClassType::from_field_class_type(class) else {
            return Err((self, class));
        };

        Ok(match matched {
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

    #[must_use]
    pub fn into_uint(self) -> BtFieldUnsignedIntegerConst {
        match self.cast() {
            Ok(BtFieldType::UnsignedInteger(inner)) => inner,
            Ok(t) => panic!("Expected unsigned integer, got {:?}", t.get_class_type()),
            _ => panic!("Expected unsigned integer, got unsupported type"),
        }
    }

    #[must_use]
    pub fn into_int(self) -> BtFieldSignedIntegerConst {
        match self.cast() {
            Ok(BtFieldType::SignedInteger(inner)) => inner,
            Ok(t) => panic!("Expected signed integer, got {:?}", t.get_class_type()),
            _ => panic!("Expected signed integer, got unsupported type"),
        }
    }

    #[must_use]
    pub fn into_string(self) -> BtFieldStringConst {
        match self.cast() {
            Ok(BtFieldType::String(inner)) => inner,
            Ok(t) => panic!("Expected string, got {:?}", t.get_class_type()),
            _ => panic!("Expected string, got unsupported type"),
        }
    }

    #[must_use]
    pub fn into_struct(self) -> BtFieldStructureConst {
        match self.cast() {
            Ok(BtFieldType::Structure(inner)) => inner,
            Ok(t) => panic!("Expected structure, got {:?}", t.get_class_type()),
            _ => panic!("Expected structure, got unsupported type"),
        }
    }

    #[must_use]
    pub fn into_array(self) -> BtFieldArrayConst {
        match self.cast() {
            Ok(BtFieldType::Array(inner)) => inner,
            Ok(t) => panic!("Expected array, got {:?}", t.get_class_type()),
            _ => panic!("Expected array, got unsupported type"),
        }
    }

    pub(crate) unsafe fn clone_unchecked(&self) -> Self {
        unsafe { BtFieldConst::new_unchecked(self.as_ptr()) }
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
            BtFieldType::UnsignedInteger(inner) => inner.fmt(f),
            BtFieldType::SignedInteger(inner) => inner.fmt(f),
            BtFieldType::String(inner) => inner.fmt(f),
            BtFieldType::Array(inner) => inner.fmt(f),
            BtFieldType::Structure(inner) => inner.fmt(f),
        }
    }
}

impl Deref for BtFieldType {
    type Target = BtFieldConst;

    fn deref(&self) -> &Self::Target {
        match self {
            BtFieldType::UnsignedInteger(inner) => inner,
            BtFieldType::SignedInteger(inner) => inner,
            BtFieldType::String(inner) => inner,
            BtFieldType::Array(inner) => inner,
            BtFieldType::Structure(inner) => inner,
        }
    }
}

macro_rules! impl_deref_for_field {
    ($type:ty) => {
        impl Deref for $type {
            type Target = BtFieldConst;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
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

impl BtFieldUnsignedIntegerConst {
    pub fn get_value(&self) -> u64 {
        unsafe { bt_field_integer_unsigned_get_value(self.as_ptr()) }
    }
}

impl_deref_for_field!(BtFieldUnsignedIntegerConst);
impl_debug_and_display_for_scalar_field!(BtFieldUnsignedIntegerConst);

impl BtFieldSignedIntegerConst {
    pub fn get_value(&self) -> i64 {
        unsafe { bt_field_integer_unsigned_get_value(self.as_ptr()) as i64 }
    }
}

impl_deref_for_field!(BtFieldSignedIntegerConst);
impl_debug_and_display_for_scalar_field!(BtFieldSignedIntegerConst);

impl BtFieldStringConst {
    #[must_use]
    pub fn get_value(&self) -> &str {
        unsafe {
            let length = bt_field_string_get_length(self.as_ptr());
            let ptr = bt_field_string_get_value(self.as_ptr());
            let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), length as usize);
            std::str::from_utf8(bytes)
                .expect("BtFieldStringConst::get_value(): Failed to convert bytes to str")
        }
    }
}

impl_deref_for_field!(BtFieldStringConst);
impl_debug_and_display_for_scalar_field!(BtFieldStringConst);

impl BtFieldArrayConst {
    #[must_use]
    pub fn get_length(&self) -> u64 {
        unsafe { bt_field_array_get_length(self.as_ptr()) }
    }

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

    #[must_use]
    pub fn read_byte_array<const N: usize>(&self) -> [u8; N] {
        assert!(N == self.get_length().try_into().unwrap());
        let mut array = [0; N];

        for (i, dest) in array.iter_mut().enumerate() {
            *dest = self
                .get_value(i as u64)
                .into_uint()
                .get_value()
                .try_into()
                .unwrap();
        }

        array
    }
}

impl_deref_for_field!(BtFieldArrayConst);

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

    #[must_use]
    pub fn get_field_by_name(&self, name: &str) -> Option<BtFieldConst> {
        let name = std::ffi::CString::new(name).expect(
            "BtFieldStructureConst::get_field_by_name(): Failed to convert name to CString",
        );

        unsafe {
            let field =
                bt_field_structure_borrow_member_field_by_name_const(self.as_ptr(), name.as_ptr());
            if field.is_null() {
                return None;
            }

            Some(BtFieldConst::new_unchecked(field))
        }
    }

    #[must_use]
    pub fn get_field_by_index(&self, index: u64) -> BtFieldConst {
        unsafe {
            BtFieldConst::new_unchecked(bt_field_array_borrow_element_field_by_index_const(
                self.as_ptr(),
                index,
            ))
        }
    }
}

impl_deref_for_field!(BtFieldStructureConst);

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
                // TODO: Bool
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
pub struct BtFieldClassConst(*const bt_field_class);

impl BtFieldClassConst {
    pub(crate) unsafe fn new_unchecked(field_class: *const bt_field_class) -> BtFieldClassConst {
        BtFieldClassConst(field_class)
    }

    fn as_ptr(&self) -> *const bt_field_class {
        self.0
    }
}

#[repr(transparent)]
pub struct BtFieldStructClassConst(BtFieldClassConst);

#[repr(transparent)]
pub struct BtFieldStructMemberClassConst(*const bt_field_class_structure_member);

impl BtFieldStructClassConst {
    #[must_use]
    pub fn get_member_count(&self) -> u64 {
        unsafe { bt_field_class_structure_get_member_count(self.as_ptr()) }
    }

    #[must_use]
    pub fn get_member_by_name(&self, name: &str) -> Option<BtFieldStructMemberClassConst> {
        let name = std::ffi::CString::new(name).expect(
            "BtFieldStructClassConst::get_field_by_name(): Failed to convert &str to CString",
        );

        unsafe {
            let field =
                bt_field_class_structure_borrow_member_by_name_const(self.as_ptr(), name.as_ptr());
            if field.is_null() {
                return None;
            }

            Some(BtFieldStructMemberClassConst::new_unchecked(field))
        }
    }

    #[must_use]
    pub fn get_member_by_index(&self, index: u64) -> BtFieldStructMemberClassConst {
        unsafe {
            BtFieldStructMemberClassConst::new_unchecked(
                bt_field_class_structure_borrow_member_by_index_const(self.as_ptr(), index),
            )
        }
    }
}

impl Deref for BtFieldStructClassConst {
    type Target = BtFieldClassConst;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BtFieldStructMemberClassConst {
    pub(crate) unsafe fn new_unchecked(
        field_class: *const bt_field_class_structure_member,
    ) -> BtFieldStructMemberClassConst {
        BtFieldStructMemberClassConst(field_class)
    }

    #[must_use]
    pub fn get_name(&self) -> &str {
        unsafe {
            let name = bt_field_class_structure_member_get_name(self.0);
            std::ffi::CStr::from_ptr(name)
                .to_str()
                .expect("Failed to convert CStr to str")
        }
    }

    #[must_use]
    pub fn get_class(&self) -> BtFieldClassConst {
        unsafe {
            BtFieldClassConst::new_unchecked(
                bt_field_class_structure_member_borrow_field_class_const(self.0),
            )
        }
    }
}

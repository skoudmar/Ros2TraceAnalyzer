use std::ptr::NonNull;

use derive_more::derive::Deref;

use crate::raw_bindings::bt_property_availability;

#[macro_export]
macro_rules! impl_deref {
    ($into:ty; for $($typ:ty),+) => {
        $(
            impl Deref for $typ {
                type Target = $into;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        )+
    };
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub(crate) struct ConstNonNull<T>(NonNull<T>);

impl<T> ConstNonNull<T> {
    pub(crate) fn new(ptr: *const T) -> Option<Self> {
        NonNull::new(ptr.cast_mut()).map(Self)
    }

    /// # Safety
    /// The caller must ensure that the pointer is not null.
    pub(crate) unsafe fn new_unchecked(ptr: *const T) -> Self {
        debug_assert!(
            !ptr.is_null(),
            "Unsafe precondition violated: pointer must not be null"
        );
        Self(NonNull::new_unchecked(ptr.cast_mut()))
    }

    #[must_use]
    pub(crate) fn as_ptr(&self) -> *const T {
        self.0.as_ptr()
    }
}

impl<T> TryFrom<*const T> for ConstNonNull<T> {
    type Error = ();

    fn try_from(ptr: *const T) -> Result<Self, Self::Error> {
        Self::new(ptr).ok_or(())
    }
}

/// A newtype for a constant value.
///
/// This is useful to prevent creating mutable references to the value.
#[repr(transparent)]
#[derive(Debug, Clone, Deref)]
pub struct Const<T>(T);

impl<T> Const<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into<U>(self) -> Const<U>
    where
        T: Into<U>,
    {
        Const::new(self.0.into())
    }

    pub fn try_into<U>(self) -> Result<Const<U>, T::Error>
    where
        T: TryInto<U>,
    {
        Ok(Const::new(self.0.try_into()?))
    }
}

pub(crate) enum BtProperyAvailabilty {
    Available,
    NotAvailable,
}

impl From<bt_property_availability> for BtProperyAvailabilty {
    fn from(value: bt_property_availability) -> Self {
        match value {
            bt_property_availability::BT_PROPERTY_AVAILABILITY_AVAILABLE => Self::Available,
            bt_property_availability::BT_PROPERTY_AVAILABILITY_NOT_AVAILABLE => Self::NotAvailable,
            _ => unreachable!("Bug: unknown bt_property_availability = {}", value.0),
        }
    }
}

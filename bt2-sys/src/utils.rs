use std::ptr::NonNull;

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

pub(crate) struct ConstNonNull<T>(NonNull<T>);

impl<T> ConstNonNull<T> {
    pub(crate) fn new(ptr: *const T) -> Option<Self> {
        NonNull::new(ptr.cast_mut()).map(Self)
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

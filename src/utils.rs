use std::fmt::{Debug, Display};
use std::sync::{Arc, Mutex, Weak};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Known<T> {
    Known(T),
    #[default]
    Unknown,
}
impl<T> Known<T> {
    pub fn new(value: T) -> Self {
        Self::Known(value)
    }

    #[inline]
    pub fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    #[inline]
    pub fn is_unknown(&self) -> bool {
        !self.is_known()
    }

    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            Self::Known(value) => value,
            Self::Unknown => panic!("Called `Known::unwrap()` on an `Unknown` value"),
        }
    }

    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Known(value) => value,
            Self::Unknown => default,
        }
    }

    pub fn eq_inner(&self, other: &T) -> bool
    where
        T: PartialEq,
    {
        match self {
            Self::Known(a) => a == other,
            Self::Unknown => false,
        }
    }

    #[inline]
    pub fn as_ref(&self) -> Known<&T> {
        match *self {
            Self::Known(ref value) => Known::Known(value),
            Self::Unknown => Known::Unknown,
        }
    }

    pub fn map<U, F>(self, f: F) -> Known<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Known(x) => Known::Known(f(x)),
            Self::Unknown => Known::Unknown,
        }
    }
}

impl<T> From<Option<T>> for Known<T> {
    #[inline]
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Known(value),
            None => Self::Unknown,
        }
    }
}

impl<T> From<T> for Known<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::Known(value)
    }
}

impl<T> From<Known<T>> for Option<T> {
    #[inline]
    fn from(value: Known<T>) -> Self {
        match value {
            Known::Known(value) => Some(value),
            Known::Unknown => None,
        }
    }
}

impl<T> std::fmt::Display for Known<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Known(value) => write!(f, "{value}"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl<T> std::fmt::LowerHex for Known<T>
where
    T: std::fmt::LowerHex,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Known(value) => value.fmt(f),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[macro_export]
macro_rules! impl_from_for_enum {
    ($(#[$attr:meta])* $enum_vis:vis enum $enum:ident { $($var_name:ident($typ:ty),)* }) => {
        $(#[$attr])*
        $enum_vis enum $enum {
            $($var_name($typ),)*
        }
        $(
            impl From<$typ> for $enum {
                fn from(value: $typ) -> Self {
                    Self::$var_name(value)
                }
            }
        )*
    };
}

pub(crate) struct DisplayArcMutex<'a, T> {
    arc: &'a Arc<Mutex<T>>,
    skip: bool,
}

impl<'a, T: Display> DisplayArcMutex<'a, T> {
    pub fn new(arc: &'a Arc<Mutex<T>>, skip: bool) -> Self {
        Self { arc, skip }
    }
}

impl<'a, T: Display> Display for DisplayArcMutex<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.skip {
            return write!(f, "...");
        }
        let inner = self.arc.lock().unwrap();
        write!(f, "{inner:#}")
    }
}

impl<'a, T: Debug> Debug for DisplayArcMutex<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.skip {
            return write!(f, "...");
        }
        let inner = self.arc.lock().unwrap();
        write!(f, "{inner:?}")
    }
}

pub(crate) struct DisplayWeakMutex<'a, T> {
    weak: &'a Weak<Mutex<T>>,
    skip: bool,
}

impl<'a, T> DisplayWeakMutex<'a, T> {
    pub fn new(weak: &'a Weak<Mutex<T>>, skip: bool) -> Self {
        Self { weak, skip }
    }
}

impl<'a, T: Display> Display for DisplayWeakMutex<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.skip {
            return write!(f, "...");
        }
        let Some(strong) = self.weak.upgrade() else {
            return write!(f, "DROPED");
        };
        let inner = strong.lock().unwrap();
        write!(f, "{inner:#}")
    }
}

impl<'a, T: Debug> Debug for DisplayWeakMutex<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.skip {
            return write!(f, "...");
        }
        let Some(strong) = self.weak.upgrade() else {
            return write!(f, "DROPED");
        };
        let inner = strong.lock().unwrap();
        write!(f, "{inner:#?}")
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DisplayDebug<T>(pub(crate) T);

impl<T: Debug> Display for DisplayDebug<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DisplayDuration(pub(crate) i64);

impl std::fmt::Display for DisplayDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let nanos = self.0;
        if nanos % 1_000_000_000 == 0 {
            write!(f, "{} s", nanos / 1_000_000_000)
        } else if nanos % 1_000_000 == 0 {
            write!(f, "{} ms", nanos / 1_000_000)
        } else if nanos % 1_000 == 0 {
            write!(f, "{} us", nanos / 1_000)
        } else {
            write!(f, "{nanos} ns")
        }
    }
}

use std::fmt::{Debug, Display};
use std::sync::{Arc, Mutex, Weak};

use derive_more::derive::From;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Known<T> {
    Known(T),
    #[default]
    Unknown,
}
impl<T> Known<T> {
    pub const fn new(value: T) -> Self {
        Self::Known(value)
    }

    #[inline]
    pub const fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    #[inline]
    pub const fn is_unknown(&self) -> bool {
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
    pub const fn as_ref(&self) -> Known<&T> {
        match *self {
            Self::Known(ref value) => Known::Known(value),
            Self::Unknown => Known::Unknown,
        }
    }

    #[inline]
    pub fn as_mut(&mut self) -> Known<&mut T> {
        match self {
            Self::Known(value) => Known::Known(value),
            Self::Unknown => Known::Unknown,
        }
    }

    #[inline]
    pub fn as_deref(&self) -> Known<&T::Target>
    where
        T: std::ops::Deref,
    {
        match self {
            Self::Known(value) => Known::Known(value),
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

    pub fn map_or<U>(self, default: U, f: impl FnOnce(T) -> U) -> U {
        match self {
            Self::Known(x) => f(x),
            Self::Unknown => default,
        }
    }

    pub(crate) fn is_unknown_or_eq<U>(&self, other: &U) -> bool
    where
        T: PartialEq<U>,
    {
        match self {
            Self::Known(value) => value == other,
            Self::Unknown => true,
        }
    }

    pub fn expect(self, msg: &str) -> T {
        match self {
            Self::Known(value) => value,
            Self::Unknown => panic!("{}", msg),
        }
    }
}

impl<T> From<Option<T>> for Known<T> {
    #[inline]
    fn from(value: Option<T>) -> Self {
        value.map_or_else(|| Self::Unknown, |value| Self::Known(value))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeakKnown<T> {
    Known(T),
    Unknown,
    Dropped,
}

impl<T> WeakKnown<T> {
    pub const fn new(value: T) -> Self {
        Self::Known(value)
    }

    #[inline]
    pub const fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    #[inline]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    #[inline]
    pub const fn is_droped(&self) -> bool {
        matches!(self, Self::Dropped)
    }

    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            Self::Known(value) => value,
            Self::Unknown => panic!("Called `WeakKnown::unwrap()` on an `Unknown` value"),
            Self::Dropped => panic!("Called `WeakKnown::unwrap()` on a `Dropped` value"),
        }
    }

    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Known(value) => value,
            Self::Unknown | Self::Dropped => default,
        }
    }

    pub fn eq_inner(&self, other: &T) -> bool
    where
        T: PartialEq,
    {
        match self {
            Self::Known(a) => a == other,
            Self::Unknown | Self::Dropped => false,
        }
    }

    #[inline]
    pub const fn as_ref(&self) -> WeakKnown<&T> {
        match *self {
            Self::Known(ref value) => WeakKnown::Known(value),
            Self::Unknown => WeakKnown::Unknown,
            Self::Dropped => WeakKnown::Dropped,
        }
    }

    #[inline]
    pub fn as_deref(&self) -> WeakKnown<&T::Target>
    where
        T: std::ops::Deref,
    {
        match self {
            Self::Known(value) => WeakKnown::Known(value),
            Self::Unknown => WeakKnown::Unknown,
            Self::Dropped => WeakKnown::Dropped,
        }
    }

    pub fn map<U, F>(self, f: F) -> WeakKnown<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Known(x) => WeakKnown::Known(f(x)),
            Self::Unknown => WeakKnown::Unknown,
            Self::Dropped => WeakKnown::Dropped,
        }
    }
}

impl<T> From<Known<T>> for WeakKnown<T> {
    #[inline]
    fn from(value: Known<T>) -> Self {
        match value {
            Known::Known(value) => Self::Known(value),
            Known::Unknown => Self::Unknown,
        }
    }
}

impl<T> From<T> for WeakKnown<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::Known(value)
    }
}

impl<T> std::fmt::Display for WeakKnown<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Known(value) => write!(f, "{value}"),
            Self::Unknown => write!(f, "Unknown"),
            Self::Dropped => write!(f, "Dropped"),
        }
    }
}

pub struct DisplayArcMutex<'a, T> {
    arc: &'a Arc<Mutex<T>>,
    skip: bool,
}

impl<'a, T: Display> DisplayArcMutex<'a, T> {
    pub const fn new(arc: &'a Arc<Mutex<T>>, skip: bool) -> Self {
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

pub struct DisplayArcWeakMutex<'a, T> {
    arc_weak: &'a ArcWeak<Mutex<T>>,
    skip: bool,
}

pub struct DisplayArcWeakMutexMapped<'a, T, F> {
    arc_weak: &'a ArcWeak<Mutex<T>>,
    display_fn: F,
}

impl<'a, T> DisplayArcWeakMutex<'a, T> {
    pub const fn new(weak: &'a ArcWeak<Mutex<T>>, skip: bool) -> Self {
        Self {
            arc_weak: weak,
            skip,
        }
    }

    pub fn map<F>(self, display_fn: F) -> DisplayArcWeakMutexMapped<'a, T, F>
    where
        F: Fn(&T, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
    {
        DisplayArcWeakMutexMapped {
            arc_weak: self.arc_weak,
            display_fn,
        }
    }
}

impl<'a, T: Display> std::fmt::Display for DisplayArcWeakMutex<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.skip {
            return write!(f, "...");
        }
        let Some(strong) = self.arc_weak.get_arc() else {
            return write!(f, "DROPPED");
        };
        let inner = strong.lock().unwrap();
        write!(f, "{inner:#}")
    }
}

impl<'a, T, F> std::fmt::Display for DisplayArcWeakMutexMapped<'a, T, F>
where
    F: Fn(&T, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some(strong) = self.arc_weak.get_arc() else {
            return write!(f, "DROPPED");
        };
        let inner = strong.lock().unwrap();
        (self.display_fn)(&inner, f)
    }
}

impl<'a, T: Debug> Debug for DisplayArcWeakMutex<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.skip {
            return write!(f, "...");
        }
        let Some(strong) = self.arc_weak.get_arc() else {
            return write!(f, "DROPPED");
        };
        let inner = strong.lock().unwrap();
        write!(f, "{inner:#?}")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayDebug<T>(pub(crate) T);

impl<T: Debug> Display for DisplayDebug<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayDuration(pub(crate) i64);

impl std::fmt::Display for DisplayDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SUFFIX: [&str; 6] = ["ns", "μs", "ms", "s", "min", "h"];
        /// Factor to convert the duration to the next suffix.
        const FACTOR: [i64; SUFFIX.len() - 1] = [1000, 1000, 1000, 60, 60];

        let mut value = self.0;
        let mut suffix = 0;
        while suffix < SUFFIX.len() - 1 && value % FACTOR[suffix] == 0 {
            value /= FACTOR[suffix];

            suffix += 1;
        }

        write!(f, "{} {}", value, SUFFIX[suffix])
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayLargeDuration(pub(crate) u128);

impl std::fmt::Display for DisplayLargeDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SUFFIX: [&str; 6] = ["ns", "μs", "ms", "s", "min", "h"];
        /// Factor to convert the duration to the next suffix.
        const FACTOR: [u128; SUFFIX.len() - 1] = [1000, 1000, 1000, 60, 60];

        let mut value = self.0;
        let mut suffix = 0;
        while suffix < SUFFIX.len() - 1 && value % FACTOR[suffix] == 0 {
            value /= FACTOR[suffix];

            suffix += 1;
        }

        write!(f, "{} {}", value, SUFFIX[suffix])
    }
}

/// Wrapper for durations represented as nanoseconds for display purposes.
///
/// This wrapper is imprecise and should not be used for calculations.
///
/// Note: The display of years is assuming every year has 365 days.
#[derive(Debug, Clone, Copy, From)]
pub struct DurationDisplayImprecise(pub i64);

impl std::fmt::Display for DurationDisplayImprecise {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SUFFIX: [&str; 8] = ["ns", "μs", "ms", "s", "min", "h", "days", "years"];
        /// Factor to convert the duration to the next suffix.
        const FACTOR: [i64; SUFFIX.len() - 1] = [1000, 1000, 1000, 60, 60, 24, 365];

        let mut value = self.0;
        let mut suffix = 0;
        let mut total_factor = 1;
        while suffix < SUFFIX.len() - 1 && value >= FACTOR[suffix] {
            value /= FACTOR[suffix];
            total_factor *= FACTOR[suffix];

            suffix += 1;
        }

        let value = self.0 as f64 / total_factor as f64;

        write!(f, "{} {}", value, SUFFIX[suffix])
    }
}

pub struct DebugOptionHex<'a, T>(pub &'a Option<T>);

impl<'a, T: std::fmt::LowerHex> std::fmt::Debug for DebugOptionHex<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(value) => f
                .debug_tuple("Some")
                .field(&format_args!("{value:#x}"))
                .finish(),
            None => f.debug_struct("None").finish(),
        }
    }
}

#[derive(Debug)]
pub enum ArcWeak<T> {
    Arc(Arc<T>),
    Weak(Weak<T>),
}

impl<T> ArcWeak<T> {
    pub fn get_arc(&self) -> Option<Arc<T>> {
        match self {
            Self::Arc(arc) => Some(Arc::clone(arc)),
            Self::Weak(weak) => weak.upgrade(),
        }
    }

    /// Upgrade the `ArcWeak` to an `Arc` in place.
    pub fn upgrade_in_place(&mut self) -> bool {
        match self {
            Self::Arc(_) => true,
            Self::Weak(weak) => {
                if let Some(arc) = weak.upgrade() {
                    *self = Self::Arc(arc);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Downgrade the `ArcWeak` to a `Weak` in place.
    pub fn downgrade_in_place(&mut self) {
        match self {
            Self::Arc(arc) => {
                let weak = Arc::downgrade(arc);
                *self = Self::Weak(weak);
            }
            Self::Weak(_) => {}
        }
    }

    pub(crate) fn get_weak(&self) -> Weak<T> {
        match self {
            Self::Arc(arc) => Arc::downgrade(arc),
            Self::Weak(weak) => weak.clone(),
        }
    }
}

impl<T> Clone for ArcWeak<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Arc(arc) => Self::Arc(Arc::clone(arc)),
            Self::Weak(weak) => Self::Weak(Weak::clone(weak)),
        }
    }
}

impl<T> From<Arc<T>> for ArcWeak<T> {
    fn from(arc: Arc<T>) -> Self {
        Self::Arc(arc)
    }
}

impl<T> From<Weak<T>> for ArcWeak<T> {
    fn from(weak: Weak<T>) -> Self {
        Self::Weak(weak)
    }
}

pub trait CyclicDependency {
    fn break_cycle(&mut self);

    fn create_cycle(&mut self) -> bool;
}

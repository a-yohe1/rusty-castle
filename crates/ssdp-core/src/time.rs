//! Monotonic time abstraction for sans-IO state machines.
//!
//! `Instant` is a newtype over [`core::time::Duration`] representing elapsed time since an
//! arbitrary monotonic reference point chosen by the caller.  The library never calls
//! `Instant::now()`; all time values are supplied by the caller.

use core::ops::{Add, Sub};
use core::time::Duration;

/// A point in monotonic time.
///
/// The zero point is arbitrary; callers must use a consistent reference across all calls
/// into the same state machine instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(Duration);

impl Instant {
    /// The zero instant (reference point).
    pub const ZERO: Self = Self(Duration::ZERO);

    /// Creates an instant from a duration since the reference point.
    #[inline]
    pub const fn from_duration(d: Duration) -> Self {
        Self(d)
    }

    /// Creates an instant from whole seconds since the reference point.
    #[inline]
    pub const fn from_secs(secs: u64) -> Self {
        Self(Duration::from_secs(secs))
    }

    /// Creates an instant from milliseconds since the reference point.
    #[inline]
    pub const fn from_millis(ms: u64) -> Self {
        Self(Duration::from_millis(ms))
    }

    /// Returns the underlying duration since the reference point.
    #[inline]
    pub const fn as_duration(self) -> Duration {
        self.0
    }

    /// Saturating addition with a duration.
    #[inline]
    pub fn checked_add(self, d: Duration) -> Option<Self> {
        self.0.checked_add(d).map(Self)
    }

    /// Returns the duration elapsed between `earlier` and `self`, or `None` if
    /// `self < earlier`.
    #[inline]
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        self.0.checked_sub(earlier.0)
    }
}

impl Add<Duration> for Instant {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Duration) -> Self {
        Self(self.0 + rhs)
    }
}

impl Sub<Duration> for Instant {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Duration) -> Self {
        Self(self.0 - rhs)
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;
    #[inline]
    fn sub(self, rhs: Instant) -> Duration {
        self.0 - rhs.0
    }
}

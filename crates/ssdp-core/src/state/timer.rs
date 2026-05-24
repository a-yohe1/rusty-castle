//! Timer tracking for SSDP state machines.

use crate::time::Instant;

/// A scheduled wakeup point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Timer {
    pub(crate) fire_at: Instant,
    pub(crate) kind: TimerKind,
}

/// The purpose of a pending timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimerKind {
    /// Periodic alive re-advertisement (Device).
    AliveRefresh,
    /// Initial burst alive (Device, for initial count tracking).
    InitialAlive,
    /// Delayed M-SEARCH response with jitter (Device).
    SearchResponse,
    /// Periodic M-SEARCH retransmission (ControlPoint).
    SearchRetransmit,
    /// Cache entry expiry (ControlPoint).
    CacheExpiry,
}

/// A small fixed-capacity priority queue of timers (min-heap by fire_at).
///
/// `N` is the maximum number of concurrent timers.
pub(crate) struct TimerQueue<const N: usize> {
    entries: [Option<Timer>; N],
}

impl<const N: usize> TimerQueue<N> {
    pub(crate) const fn new() -> Self {
        Self { entries: [None; N] }
    }

    /// Inserts a timer, replacing an existing entry of the same kind if present.
    pub(crate) fn set(&mut self, t: Timer) {
        // Replace existing entry of same kind.
        for e in self.entries.iter_mut().flatten() {
            if e.kind == t.kind {
                *e = t;
                return;
            }
        }
        // Insert into first empty slot.
        for slot in self.entries.iter_mut() {
            if slot.is_none() {
                *slot = Some(t);
                return;
            }
        }
        // No room — overwrite the furthest-future timer (drop least urgent).
        if let Some(slot) = self.entries.iter_mut().max_by_key(|s| s.map(|e| e.fire_at)) {
            *slot = Some(t);
        }
    }

    /// Cancels any timer of the given kind.
    pub(crate) fn cancel(&mut self, kind: TimerKind) {
        for slot in self.entries.iter_mut() {
            if slot.map(|e| e.kind) == Some(kind) {
                *slot = None;
            }
        }
    }

    /// Returns the earliest fire time, if any timers are scheduled.
    pub(crate) fn next_timeout(&self) -> Option<Instant> {
        self.entries
            .iter()
            .filter_map(|s| *s)
            .map(|e| e.fire_at)
            .min()
    }

    /// Removes and returns all timers whose `fire_at <= now`.
    pub(crate) fn drain_expired(&mut self, now: Instant) -> TimerDrain<'_, N> {
        TimerDrain { queue: self, now }
    }
}

pub(crate) struct TimerDrain<'a, const N: usize> {
    queue: &'a mut TimerQueue<N>,
    now: Instant,
}

impl<const N: usize> Iterator for TimerDrain<'_, N> {
    type Item = TimerKind;
    fn next(&mut self) -> Option<Self::Item> {
        for slot in self.queue.entries.iter_mut() {
            if let Some(e) = slot {
                if e.fire_at <= self.now {
                    let kind = e.kind;
                    *slot = None;
                    return Some(kind);
                }
            }
        }
        None
    }
}

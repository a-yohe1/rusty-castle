//! Discovered-service cache for the ControlPoint state machine.
//!
//! Maps USN strings to cache entries and expires them based on max-age.

use crate::time::Instant;
use core::time::Duration;
use heapless::FnvIndexMap;

/// Maximum number of simultaneously cached SSDP entries.
pub const DEFAULT_CACHE_CAPACITY: usize = 64;

/// A single cached SSDP service entry.
#[derive(Clone, Debug)]
pub struct CacheEntry<const LOC: usize, const USN: usize> {
    /// The LOCATION URL.
    pub location: heapless::String<LOC>,
    /// The USN (for identification / dedup).
    pub usn: heapless::String<USN>,
    /// The time at which this entry expires.
    pub expires_at: Instant,
}

/// SSDP service cache backed by a heapless map.
///
/// `CAP` is the maximum number of entries; `LOC` is the max location string length;
/// `USN` is the max USN string length.
pub struct ServiceCache<const CAP: usize, const LOC: usize, const USN: usize> {
    /// USN string → cache entry.
    entries: FnvIndexMap<heapless::String<USN>, CacheEntry<LOC, USN>, CAP>,
}

impl<const CAP: usize, const LOC: usize, const USN: usize> ServiceCache<CAP, LOC, USN> {
    /// Creates an empty cache.
    pub const fn new() -> Self {
        Self {
            entries: FnvIndexMap::new(),
        }
    }

    /// Inserts or refreshes an entry.
    ///
    /// Returns `false` if the USN or LOCATION string is too long, or the cache is full.
    pub fn insert(
        &mut self,
        usn_str: &str,
        location_str: &str,
        max_age: Duration,
        now: Instant,
    ) -> bool {
        let Ok(usn_key) = heapless::String::<USN>::try_from(usn_str) else {
            return false;
        };
        let Ok(loc) = heapless::String::<LOC>::try_from(location_str) else {
            return false;
        };
        let Ok(usn_copy) = heapless::String::<USN>::try_from(usn_str) else {
            return false;
        };
        let expires_at = now + max_age;
        let entry = CacheEntry {
            location: loc,
            usn: usn_copy,
            expires_at,
        };
        // insert or update
        self.entries.insert(usn_key, entry).is_ok()
    }

    /// Removes an entry by USN.
    pub fn remove(&mut self, usn_str: &str) -> bool {
        if let Ok(key) = heapless::String::<USN>::try_from(usn_str) {
            self.entries.remove(&key).is_some()
        } else {
            false
        }
    }

    /// Returns the entry for the given USN, if present.
    pub fn get(&self, usn_str: &str) -> Option<&CacheEntry<LOC, USN>> {
        heapless::String::<USN>::try_from(usn_str)
            .ok()
            .and_then(|k| self.entries.get(&k))
    }

    /// Removes all entries whose `expires_at <= now` and calls `on_expired` for each.
    pub fn expire<F: FnMut(&str)>(&mut self, now: Instant, mut on_expired: F) {
        // Collect keys to remove (can't mutate while iterating).
        let mut to_remove: heapless::Vec<heapless::String<USN>, CAP> = heapless::Vec::new();
        for (k, v) in self.entries.iter() {
            if v.expires_at <= now {
                let _ = to_remove.push(k.clone());
            }
        }
        for k in to_remove.iter() {
            self.entries.remove(k);
            on_expired(k.as_str());
        }
    }

    /// Returns the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the earliest expiry time, if any entries are cached.
    pub fn next_expiry(&self) -> Option<Instant> {
        self.entries.values().map(|e| e.expires_at).min()
    }
}

impl<const CAP: usize, const LOC: usize, const USN: usize> Default for ServiceCache<CAP, LOC, USN> {
    fn default() -> Self {
        Self::new()
    }
}

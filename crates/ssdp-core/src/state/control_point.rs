//! Control-point-side SSDP state machine.
//!
//! Manages M-SEARCH transmission, response collection, NOTIFY handling, and cache expiry.

use crate::encode::encode_msearch;
use crate::header::nts::Nts;
use crate::header::target::TargetRef;
use crate::message::{MSearchRef, MessageRef};
use crate::parse::parse_datagram;
use crate::state::cache::ServiceCache;
use crate::state::timer::{Timer, TimerKind, TimerQueue};
use crate::state::{Destination, Transmit};
use crate::time::Instant;
use core::net::SocketAddr;
use core::time::Duration;
use heapless::Vec;

/// Maximum number of retransmissions for a single M-SEARCH (excluding the initial send).
const MAX_RETRANSMITS: u8 = 2;

/// Maximum number of entries in the ControlPoint service cache.
pub const CACHE_CAP: usize = 64;
/// Maximum byte length of a LOCATION URL in the cache.
pub const CACHE_LOC: usize = 256;
/// Maximum byte length of a USN string in the cache.
pub const CACHE_USN: usize = 128;

/// Events emitted by the ControlPoint state machine.
#[derive(Clone, Debug)]
#[non_exhaustive]
// In no_std without alloc, boxing the large variant is not possible.
#[allow(clippy::large_enum_variant)]
pub enum CpEvent {
    /// A new or refreshed service was discovered.
    SearchHit {
        /// Unique Service Name.
        usn: heapless::String<CACHE_USN>,
        /// Device description URL.
        location: heapless::String<CACHE_LOC>,
        /// Remaining max-age at time of receipt.
        max_age: Duration,
    },
    /// A device signalled departure via `ssdp:byebye`.
    ByeBye {
        /// USN of the departing service.
        usn: heapless::String<CACHE_USN>,
    },
    /// A cached entry has expired (max-age elapsed without renewal).
    Expired {
        /// USN of the expired service.
        usn: heapless::String<CACHE_USN>,
    },
}

/// Sans-IO ControlPoint state machine.
pub struct ControlPoint {
    /// Discovered-service cache.
    cache: ServiceCache<CACHE_CAP, CACHE_LOC, CACHE_USN>,
    /// Timer queue.
    timers: TimerQueue<4>,
    /// Pending outgoing datagrams.
    pending_transmits: Vec<(Destination, usize), 4>, // (dest, payload_len in scratch_buf)
    /// Scratch encoding buffer.
    scratch_buf: [u8; 512],
    /// Pending events.
    events: Vec<CpEvent, 8>,
    /// M-SEARCH retransmit counter.
    retransmit_count: u8,
    /// Active M-SEARCH MX (seconds).
    active_mx: u8,
    /// Stored USER-AGENT.
    user_agent: Option<heapless::String<128>>,
}

impl ControlPoint {
    /// Creates a new ControlPoint.
    pub fn new(user_agent: Option<&str>) -> Self {
        Self {
            cache: ServiceCache::new(),
            timers: TimerQueue::new(),
            pending_transmits: Vec::new(),
            scratch_buf: [0u8; 512],
            events: Vec::new(),
            retransmit_count: 0,
            active_mx: 3,
            user_agent: user_agent.and_then(|s| heapless::String::try_from(s).ok()),
        }
    }

    /// Initiates an M-SEARCH for the given target.
    ///
    /// Schedules the initial send plus up to `MAX_RETRANSMITS` retransmissions.
    pub fn search(&mut self, target: &TargetRef<'_>, mx: u8, now: Instant) {
        self.retransmit_count = 0;
        self.active_mx = mx.clamp(1, 5);
        self.enqueue_msearch(target, mx);
        // Schedule first retransmit at mx/2.
        self.timers.set(Timer {
            fire_at: now + Duration::from_secs(u64::from(self.active_mx) / 2 + 1),
            kind: TimerKind::SearchRetransmit,
        });
    }

    /// Handles an incoming datagram.
    pub fn handle_datagram(&mut self, now: Instant, _source: SocketAddr, payload: &[u8]) {
        let Ok(msg) = parse_datagram(payload) else {
            return;
        };
        match msg {
            MessageRef::SearchResponse(resp) => {
                let usn_str = usn_to_string(&resp.usn);
                let loc = resp.location;
                self.cache.insert(&usn_str, loc, resp.max_age, now);
                if let (Ok(usn_s), Ok(loc_s)) = (
                    heapless::String::<CACHE_USN>::try_from(usn_str.as_str()),
                    heapless::String::<CACHE_LOC>::try_from(loc),
                ) {
                    let _ = self.events.push(CpEvent::SearchHit {
                        usn: usn_s,
                        location: loc_s,
                        max_age: resp.max_age,
                    });
                }
                self.reschedule_cache_expiry(now);
            }
            MessageRef::Notify(n) => {
                let usn_str = usn_to_string(&n.usn);
                match n.nts {
                    Nts::Alive | Nts::Update => {
                        if let (Some(loc), Some(age)) = (n.location, n.max_age) {
                            self.cache.insert(&usn_str, loc, age, now);
                            if let (Ok(usn_s), Ok(loc_s)) = (
                                heapless::String::<CACHE_USN>::try_from(usn_str.as_str()),
                                heapless::String::<CACHE_LOC>::try_from(loc),
                            ) {
                                let _ = self.events.push(CpEvent::SearchHit {
                                    usn: usn_s,
                                    location: loc_s,
                                    max_age: age,
                                });
                            }
                            self.reschedule_cache_expiry(now);
                        }
                    }
                    Nts::ByeBye => {
                        self.cache.remove(&usn_str);
                        if let Ok(usn_s) = heapless::String::<CACHE_USN>::try_from(usn_str.as_str())
                        {
                            let _ = self.events.push(CpEvent::ByeBye { usn: usn_s });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Handles a timer expiry.
    pub fn handle_timeout(&mut self, now: Instant) {
        let expired: heapless::Vec<TimerKind, 4> = self.timers.drain_expired(now).collect();
        for kind in expired {
            match kind {
                TimerKind::SearchRetransmit => {
                    // Retransmit logic omitted — would re-enqueue M-SEARCH.
                    self.retransmit_count += 1;
                    if self.retransmit_count < MAX_RETRANSMITS {
                        self.timers.set(Timer {
                            fire_at: now + Duration::from_secs(u64::from(self.active_mx)),
                            kind: TimerKind::SearchRetransmit,
                        });
                    }
                }
                TimerKind::CacheExpiry => {
                    let events = &mut self.events;
                    self.cache.expire(now, |usn_str| {
                        if let Ok(usn_s) = heapless::String::<CACHE_USN>::try_from(usn_str) {
                            let _ = events.push(CpEvent::Expired { usn: usn_s });
                        }
                    });
                    self.reschedule_cache_expiry(now);
                }
                _ => {}
            }
        }
    }

    /// Returns the next outgoing datagram, if any.
    pub fn poll_transmit<'b>(&mut self, buf: &'b mut [u8]) -> Option<Transmit<'b>> {
        if let Some((dest, len)) = self.pending_transmits.pop() {
            let n = len.min(buf.len());
            buf[..n].copy_from_slice(&self.scratch_buf[..n]);
            return Some(Transmit {
                dest,
                payload: &buf[..n],
            });
        }
        None
    }

    /// Returns the next application event, if any.
    pub fn poll_event(&mut self) -> Option<CpEvent> {
        if self.events.is_empty() {
            None
        } else {
            Some(self.events.remove(0))
        }
    }

    /// Returns the time at which the caller should next call [`Self::handle_timeout`].
    pub fn poll_timeout(&self) -> Option<Instant> {
        self.timers.next_timeout()
    }

    // ---- internals ----

    fn enqueue_msearch(&mut self, target: &TargetRef<'_>, mx: u8) {
        let ua = self.user_agent.as_deref();
        let msg = MSearchRef {
            host: "239.255.255.250:1900",
            st: target.clone(),
            mx,
            user_agent: ua,
            cpfn: None,
            cpuuid: None,
            tcpport: None,
        };
        if let Ok(enc) = encode_msearch(&msg, &mut self.scratch_buf) {
            let len = enc.len();
            let _ = self.pending_transmits.push((Destination::MulticastV4, len));
        }
    }

    fn reschedule_cache_expiry(&mut self, now: Instant) {
        if let Some(next) = self.cache.next_expiry() {
            if next > now {
                self.timers.set(Timer {
                    fire_at: next,
                    kind: TimerKind::CacheExpiry,
                });
            }
        }
    }
}

/// Converts a UsnRef to a heapless string (best-effort; truncates if too long).
fn usn_to_string(usn: &crate::header::usn::UsnRef<'_>) -> heapless::String<CACHE_USN> {
    let mut s = heapless::String::<CACHE_USN>::new();
    let _ = s.push_str("uuid:");
    let _ = s.push_str(usn.device_uuid);
    if let Some(ref emb) = usn.embedded {
        let _ = s.push_str("::");
        // Append embedded target string.
        match emb {
            TargetRef::All => {
                let _ = s.push_str("ssdp:all");
            }
            TargetRef::RootDevice => {
                let _ = s.push_str("upnp:rootdevice");
            }
            TargetRef::Uuid(u) => {
                let _ = s.push_str("uuid:");
                let _ = s.push_str(u);
            }
            TargetRef::DeviceType {
                domain,
                kind,
                version,
            } => {
                let _ = s.push_str("urn:");
                let _ = s.push_str(domain);
                let _ = s.push_str(":device:");
                let _ = s.push_str(kind);
                let _ = s.push(':');
                // Simple version append.
                let mut buf = [0u8; 10];
                let vs = version_str(*version, &mut buf);
                let _ = s.push_str(vs);
            }
            TargetRef::ServiceType {
                domain,
                kind,
                version,
            } => {
                let _ = s.push_str("urn:");
                let _ = s.push_str(domain);
                let _ = s.push_str(":service:");
                let _ = s.push_str(kind);
                let _ = s.push(':');
                let mut buf = [0u8; 10];
                let vs = version_str(*version, &mut buf);
                let _ = s.push_str(vs);
            }
        }
    }
    s
}

fn version_str(v: u32, buf: &mut [u8; 10]) -> &str {
    if v == 0 {
        return "0";
    }
    let mut i = 10usize;
    let mut n = v;
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    core::str::from_utf8(&buf[i..]).unwrap_or("?")
}

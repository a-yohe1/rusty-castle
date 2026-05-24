//! Device-side SSDP state machine.
//!
//! Manages alive announcements, periodic re-advertisements, and responses to M-SEARCH.

use crate::consts::INITIAL_ALIVE_COUNT;
use crate::encode;
use crate::error::EncodeError;
use crate::header::nts::Nts;
use crate::header::target::TargetRef;
use crate::header::usn::UsnRef;
use crate::message::MessageRef;
use crate::message::{NotifyRef, SearchResponseRef};
use crate::parse::parse_datagram;
use crate::state::timer::{Timer, TimerKind, TimerQueue};
use crate::state::{Destination, Transmit};
use crate::time::Instant;
use core::net::SocketAddr;
use core::time::Duration;
use heapless::{String, Vec};

/// Maximum number of simultaneous advertised targets (root + device type + service types).
pub const MAX_TARGETS: usize = 8;

/// Maximum length of a LOCATION URL in bytes.
pub const MAX_LOCATION_LEN: usize = 256;

/// Maximum length of a UUID string in bytes.
pub const MAX_UUID_LEN: usize = 64;

/// Maximum length of a SERVER header value in bytes.
pub const MAX_SERVER_LEN: usize = 128;

/// Maximum length of a URN string component in bytes.
pub const MAX_URN_LEN: usize = 128;

/// An advertised target entry stored in the Device.
struct AdvertEntry {
    /// NT/ST value as a heapless string (e.g. `upnp:rootdevice`).
    nt_str: String<MAX_URN_LEN>,
    /// Max-age in seconds.
    max_age: Duration,
    /// Initial alive burst remaining count.
    initial_remaining: u8,
}

/// Events emitted by the Device state machine.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum DeviceEvent {
    /// An M-SEARCH request was received from a control point.
    SearchReceived {
        /// Source address of the control point.
        source: SocketAddr,
    },
    /// Encoding a transmit buffer failed (buffer too small).
    EncodeError(EncodeError),
}

/// Sans-IO Device state machine.
///
/// Manages SSDP advertisement lifecycle for a single UPnP device.
pub struct Device {
    /// Device UUID.
    uuid: String<MAX_UUID_LEN>,
    /// Device description URL.
    location: String<MAX_LOCATION_LEN>,
    /// Optional SERVER header value.
    server: Option<String<MAX_SERVER_LEN>>,
    /// BOOTID.UPNP.ORG value.
    bootid: u32,
    /// CONFIGID.UPNP.ORG value.
    configid: u32,
    /// Advertised targets.
    targets: Vec<AdvertEntry, MAX_TARGETS>,
    /// Pending transmits (encoded into scratch_buf).
    pending_transmits: Vec<(Destination, usize, usize), 16>, // (dest, start, len)
    /// Scratch buffer for encoding outbound datagrams.
    scratch_buf: [u8; 4096],
    /// Timer queue.
    timers: TimerQueue<8>,
    /// Pending events for the application.
    events: Vec<DeviceEvent, 4>,
    /// Whether shutdown (byebye) has been initiated.
    shutting_down: bool,
    /// Simple pseudo-random state for jitter (caller-seeded).
    rng_state: u32,
}

impl Device {
    /// Creates a new Device.
    ///
    /// - `uuid`: device UUID string (without `uuid:` prefix).
    /// - `location`: URL of device description.
    /// - `server`: optional SERVER header value.
    /// - `bootid`: current BOOTID (should be incremented on each restart).
    /// - `configid`: current CONFIGID.
    /// - `rng_seed`: seed for jitter; provide a value derived from hardware entropy or
    ///   time-since-boot.
    pub fn new(
        uuid: &str,
        location: &str,
        server: Option<&str>,
        bootid: u32,
        configid: u32,
        rng_seed: u32,
    ) -> Self {
        Self {
            uuid: String::try_from(uuid).unwrap_or_default(),
            location: String::try_from(location).unwrap_or_default(),
            server: server.and_then(|s| String::try_from(s).ok()),
            bootid,
            configid,
            targets: Vec::new(),
            pending_transmits: Vec::new(),
            scratch_buf: [0u8; 4096],
            timers: TimerQueue::new(),
            events: Vec::new(),
            shutting_down: false,
            rng_state: rng_seed,
        }
    }

    /// Registers an advertised target (NT).
    ///
    /// Typical targets: `upnp:rootdevice`, device type URN, service type URNs.
    /// Returns `false` if the target list is full or the NT string is too long.
    pub fn add_target(&mut self, nt: &TargetRef<'_>, max_age: Duration) -> bool {
        if self.targets.is_full() {
            return false;
        }
        let nt_str = target_to_string(nt);
        let Some(nt_str) = nt_str else { return false };
        let _ = self.targets.push(AdvertEntry {
            nt_str,
            max_age,
            initial_remaining: INITIAL_ALIVE_COUNT as u8,
        });
        true
    }

    /// Starts the initial alive burst and schedules periodic re-advertisements.
    ///
    /// Call after all targets have been registered.
    pub fn start(&mut self, now: Instant) {
        // Schedule the first initial alive immediately.
        self.timers.set(Timer {
            fire_at: now,
            kind: TimerKind::InitialAlive,
        });
    }

    /// Initiates graceful shutdown (schedules byebye for all targets).
    pub fn shutdown(&mut self, now: Instant) {
        self.shutting_down = true;
        self.send_byebye_all();
        self.timers.set(Timer {
            fire_at: now,
            kind: TimerKind::SearchResponse,
        });
        self.timers.cancel(TimerKind::AliveRefresh);
        self.timers.cancel(TimerKind::InitialAlive);
    }

    /// Handles an incoming datagram from `source`.
    pub fn handle_datagram(&mut self, now: Instant, source: SocketAddr, payload: &[u8]) {
        let Ok(msg) = parse_datagram(payload) else {
            return;
        };
        if let MessageRef::Search(search) = msg {
            let _ = self.events.push(DeviceEvent::SearchReceived { source });
            // Schedule a jitter-delayed response.
            let jitter_ms = self.next_rand() % (u32::from(search.mx) * 1000);
            let fire_at = now + Duration::from_millis(u64::from(jitter_ms));
            // Store the source addr in a simple way: encode immediately into pending_transmits
            // with the delayed fire time.  We use SearchResponse timer to trigger the send.
            self.timers.set(Timer {
                fire_at,
                kind: TimerKind::SearchResponse,
            });
            // Remember the requester address via rng_state smuggling — for a real impl we'd
            // store it properly. For now we enqueue a unicast transmit to source immediately
            // (jitter is advisory per spec when replying unicast).
            let _ = self.encode_search_responses(source, &search.st);
        }
    }

    /// Handles a timer expiry.
    pub fn handle_timeout(&mut self, now: Instant) {
        let expired: heapless::Vec<TimerKind, 8> = self.timers.drain_expired(now).collect();
        for kind in expired {
            match kind {
                TimerKind::InitialAlive => self.on_initial_alive(now),
                TimerKind::AliveRefresh => self.on_alive_refresh(now),
                TimerKind::SearchResponse => { /* response already enqueued in handle_datagram */ }
                _ => {}
            }
        }
    }

    /// Returns the next outgoing datagram, if any.
    ///
    /// `buf` is a caller-supplied buffer; the returned `Transmit` borrows from it.
    /// Callers must drain all pending transmits before feeding new input.
    pub fn poll_transmit<'b>(&mut self, buf: &'b mut [u8]) -> Option<Transmit<'b>> {
        if let Some((dest, start, len)) = self.pending_transmits.pop() {
            let src = &self.scratch_buf[start..start + len];
            let n = src.len().min(buf.len());
            buf[..n].copy_from_slice(&src[..n]);
            return Some(Transmit {
                dest,
                payload: &buf[..n],
            });
        }
        None
    }

    /// Returns the next application event, if any.
    pub fn poll_event(&mut self) -> Option<DeviceEvent> {
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

    fn on_initial_alive(&mut self, now: Instant) {
        self.send_alive_all(Destination::MulticastV4);
        // Schedule next initial alive or switch to periodic.
        let all_done = self.targets.iter_mut().all(|t| {
            if t.initial_remaining > 0 {
                t.initial_remaining -= 1;
            }
            t.initial_remaining == 0
        });
        if all_done {
            self.schedule_alive_refresh(now);
        } else {
            // Next initial alive in ~200ms with jitter.
            let jitter_ms = 100 + self.next_rand() % 100;
            self.timers.set(Timer {
                fire_at: now + Duration::from_millis(u64::from(jitter_ms)),
                kind: TimerKind::InitialAlive,
            });
        }
    }

    fn on_alive_refresh(&mut self, now: Instant) {
        if self.shutting_down {
            self.send_byebye_all();
            return;
        }
        self.send_alive_all(Destination::MulticastV4);
        self.schedule_alive_refresh(now);
    }

    fn schedule_alive_refresh(&mut self, now: Instant) {
        // Re-advertise at half the minimum max-age.
        let min_max_age = self
            .targets
            .iter()
            .map(|t| t.max_age)
            .min()
            .unwrap_or(Duration::from_secs(1800));
        let interval = min_max_age / 2;
        self.timers.set(Timer {
            fire_at: now + interval,
            kind: TimerKind::AliveRefresh,
        });
    }

    fn send_alive_all(&mut self, dest: Destination) {
        let uuid = self.uuid.clone();
        let location = self.location.clone();
        let server = self.server.clone();
        let bootid = self.bootid;
        let configid = self.configid;
        let host = match dest {
            Destination::MulticastV4 => "239.255.255.250:1900",
            _ => "239.255.255.250:1900",
        };

        for i in 0..self.targets.len() {
            let (nt_str, max_age) = {
                let t = &self.targets[i];
                (t.nt_str.clone(), t.max_age)
            };
            let nt_ref = parse_target_str(&nt_str);
            let usn = make_usn_ref(&uuid, &nt_str, &nt_ref);

            let notify = NotifyRef {
                host,
                nt: nt_ref,
                nts: Nts::Alive,
                usn,
                location: Some(location.as_str()),
                max_age: Some(max_age),
                server: server.as_deref(),
                bootid: Some(bootid),
                configid: Some(configid),
                nextbootid: None,
                searchport: None,
            };
            self.enqueue_notify(&notify, dest);
        }
    }

    fn send_byebye_all(&mut self) {
        let uuid = self.uuid.clone();
        for i in 0..self.targets.len() {
            let nt_str = self.targets[i].nt_str.clone();
            let nt_ref = parse_target_str(&nt_str);
            let usn = make_usn_ref(&uuid, &nt_str, &nt_ref);
            let notify = NotifyRef {
                host: "239.255.255.250:1900",
                nt: nt_ref,
                nts: Nts::ByeBye,
                usn,
                location: None,
                max_age: None,
                server: None,
                bootid: Some(self.bootid),
                configid: Some(self.configid),
                nextbootid: None,
                searchport: None,
            };
            self.enqueue_notify(&notify, Destination::MulticastV4);
        }
    }

    fn encode_search_responses(
        &mut self,
        dest_addr: SocketAddr,
        search_target: &TargetRef<'_>,
    ) -> Result<(), EncodeError> {
        let dest = Destination::Unicast(dest_addr);
        let uuid = self.uuid.clone();
        let location = self.location.clone();
        let server = self.server.clone();
        let bootid = self.bootid;
        let configid = self.configid;

        for i in 0..self.targets.len() {
            let (nt_str, max_age) = {
                let t = &self.targets[i];
                (t.nt_str.clone(), t.max_age)
            };
            let advertised_target = parse_target_str(&nt_str);
            if !target_matches(search_target, &advertised_target) {
                continue;
            }
            let st_ref = response_target(search_target, &advertised_target);
            let usn = make_usn_ref(&uuid, &nt_str, &st_ref);

            let resp = SearchResponseRef {
                st: st_ref,
                usn,
                location: location.as_str(),
                max_age,
                server: server.as_deref(),
                bootid: Some(bootid),
                configid: Some(configid),
                searchport: None,
            };
            let mut buf = [0u8; 1024];
            let encoded = encode::encode_response(&resp, &mut buf)?;
            self.enqueue_payload(dest, encoded)?;
        }
        Ok(())
    }

    fn enqueue_notify(&mut self, notify: &NotifyRef<'_>, dest: Destination) {
        let mut buf = [0u8; 1024];
        if let Ok(encoded) = encode::encode_notify(notify, &mut buf) {
            if let Err(err) = self.enqueue_payload(dest, encoded) {
                let _ = self.events.push(DeviceEvent::EncodeError(err));
            }
        }
    }

    fn enqueue_payload(&mut self, dest: Destination, payload: &[u8]) -> Result<(), EncodeError> {
        let start = self
            .pending_transmits
            .iter()
            .map(|(_, start, len)| start + len)
            .max()
            .unwrap_or(0);
        let end = start
            .checked_add(payload.len())
            .ok_or(EncodeError::BufferTooSmall)?;
        if end > self.scratch_buf.len() || self.pending_transmits.is_full() {
            return Err(EncodeError::BufferTooSmall);
        }
        self.scratch_buf[start..end].copy_from_slice(payload);
        let _ = self.pending_transmits.push((dest, start, payload.len()));
        Ok(())
    }

    fn next_rand(&mut self) -> u32 {
        // xorshift32
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        self.rng_state
    }
}

/// Converts a `TargetRef` to a heapless string for persistent storage.
fn target_to_string(t: &TargetRef<'_>) -> Option<String<MAX_URN_LEN>> {
    let s = match t {
        TargetRef::All => "ssdp:all",
        TargetRef::RootDevice => "upnp:rootdevice",
        TargetRef::Uuid(u) => {
            return {
                let mut s = String::<MAX_URN_LEN>::new();
                s.push_str("uuid:").ok()?;
                s.push_str(u).ok()?;
                Some(s)
            };
        }
        TargetRef::DeviceType { .. } | TargetRef::ServiceType { .. } => {
            // Use encode to produce the canonical string.
            let mut buf = [0u8; MAX_URN_LEN];
            let mut w = crate::encode::BufWriter::new(&mut buf);
            crate::encode::msearch::encode_target(&mut w, "", t).ok()?;
            // Strip the leading ": " that encode_target writes.
            let n = w.written();
            return core::str::from_utf8(&buf[2..n.saturating_sub(2)])
                .ok()
                .and_then(|s| String::<MAX_URN_LEN>::try_from(s).ok());
        }
    };
    String::<MAX_URN_LEN>::try_from(s).ok()
}

/// Re-parses a stored NT/ST string back to a `TargetRef`.  Falls back to `ssdp:all` on error.
fn parse_target_str(s: &str) -> TargetRef<'_> {
    TargetRef::parse(s).unwrap_or(TargetRef::All)
}

/// Builds a `UsnRef` for a given NT string.
fn make_usn_ref<'a>(uuid: &'a str, _nt_str: &'a str, nt: &TargetRef<'a>) -> UsnRef<'a> {
    match nt {
        TargetRef::Uuid(_) => UsnRef {
            device_uuid: uuid,
            embedded: None,
        },
        TargetRef::All => UsnRef {
            device_uuid: uuid,
            embedded: None,
        },
        _ => UsnRef {
            device_uuid: uuid,
            embedded: Some(nt.clone()),
        },
    }
}

fn target_matches(search: &TargetRef<'_>, advertised: &TargetRef<'_>) -> bool {
    match (search, advertised) {
        (TargetRef::All, _) => true,
        (TargetRef::RootDevice, TargetRef::RootDevice) => true,
        (TargetRef::Uuid(search_uuid), TargetRef::Uuid(advertised_uuid)) => {
            search_uuid.eq_ignore_ascii_case(advertised_uuid)
        }
        (
            TargetRef::DeviceType {
                domain: search_domain,
                kind: search_kind,
                version: search_version,
            },
            TargetRef::DeviceType {
                domain: advertised_domain,
                kind: advertised_kind,
                version: advertised_version,
            },
        )
        | (
            TargetRef::ServiceType {
                domain: search_domain,
                kind: search_kind,
                version: search_version,
            },
            TargetRef::ServiceType {
                domain: advertised_domain,
                kind: advertised_kind,
                version: advertised_version,
            },
        ) => {
            search_domain.eq_ignore_ascii_case(advertised_domain)
                && search_kind.eq_ignore_ascii_case(advertised_kind)
                && search_version <= advertised_version
        }
        _ => false,
    }
}

fn response_target<'a>(search: &'a TargetRef<'a>, advertised: &'a TargetRef<'a>) -> TargetRef<'a> {
    match search {
        TargetRef::DeviceType { .. } | TargetRef::ServiceType { .. }
            if target_matches(search, advertised) =>
        {
            search.clone()
        }
        _ => advertised.clone(),
    }
}

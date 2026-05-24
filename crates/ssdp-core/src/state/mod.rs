//! Sans-IO SSDP state machines.
//!
//! # Design
//!
//! Both [`device::Device`] and [`control_point::ControlPoint`] follow the quinn-proto poll pattern:
//!
//! 1. **Feed input**: call `handle_datagram` or `handle_timeout` to advance state.
//! 2. **Drain outputs**: repeatedly call `poll_transmit` (to get outbound packets),
//!    `poll_event` (to get application-visible events), and `poll_timeout` (to learn
//!    when the next wakeup is needed).
//!
//! All I/O — socket binding, multicast group joining, actual packet sending and receiving —
//! is the caller's responsibility.

pub mod cache;
pub mod control_point;
pub mod device;
pub mod timer;

use core::net::SocketAddr;

/// A destination for an outgoing SSDP datagram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    /// IPv4 SSDP multicast group `239.255.255.250:1900`.
    MulticastV4,
    /// IPv6 SSDP link-local multicast `[FF02::C]:1900`.
    MulticastV6LinkLocal,
    /// IPv6 SSDP site-local multicast `[FF05::C]:1900`.
    MulticastV6SiteLocal,
    /// Unicast to a specific address (for M-SEARCH unicast responses).
    Unicast(SocketAddr),
}

/// An outgoing datagram produced by a state machine.
#[derive(Debug)]
pub struct Transmit<'a> {
    /// Where to send the datagram.
    pub dest: Destination,
    /// The encoded datagram payload.  Valid until the next call to `poll_transmit`.
    pub payload: &'a [u8],
}

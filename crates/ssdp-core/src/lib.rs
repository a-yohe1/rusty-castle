//! Sans-IO, `no_std` SSDP (UPnP Device Architecture 1.1) protocol library.
//!
//! # Design
//!
//! - **Zero-copy parsing**: [`parse::parse_datagram`] returns [`message::MessageRef`] that
//!   borrows directly from the input buffer — no allocation required.
//! - **Buffer-based encoding**: encoders write into a caller-supplied `&mut [u8]`.
//! - **Sans-IO state machines**: [`state::device::Device`] and
//!   [`state::control_point::ControlPoint`] are pure
//!   state machines. Callers feed datagrams and timeouts in, drain transmit buffers and
//!   events out.  All I/O is the caller's responsibility.
//!
//! # Feature flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `alloc` | (reserved) Enable `alloc`-backed owned types in a future release |
//! | `std`   | Implies `alloc`; adds `std::error::Error` impls |
//! | `defmt` | Enable [`defmt`](https://crates.io/crates/defmt) logging and derives |
#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs, clippy::all)]

pub mod consts;
pub mod encode;
pub mod error;
pub mod header;
pub mod message;
pub mod parse;
pub mod state;
pub mod time;
pub(crate) mod uri;

pub use error::{EncodeError, ParseError};
pub use message::MessageRef;
pub use parse::parse_datagram;
pub use time::Instant;

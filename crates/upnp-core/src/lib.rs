//! Sans-IO, `no_std` UPnP Device Architecture primitives.
//!
//! This crate owns the protocol data model and byte/string serialization for
//! device descriptions, SCPD documents, and SOAP control messages. Runtime
//! concerns such as HTTP routing, sockets, and filesystem access live in adapter
//! crates.
#![no_std]
#![deny(unsafe_code)]
#![warn(clippy::all)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod device;
pub mod error;
pub mod ids;
pub mod scpd;
pub mod soap;
pub mod xml;

pub use error::{ParseError, WriteError};
pub use ids::{DeviceTypeRef, ServiceTypeRef, UdnRef};

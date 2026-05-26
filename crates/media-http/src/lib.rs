//! HTTP media response planning helpers.
//!
//! This crate deliberately does not bind to a concrete HTTP server. It parses
//! request metadata and returns status, headers, and byte spans that adapters can
//! translate into hyper, axum, or another runtime.
#![no_std]
#![deny(unsafe_code)]
#![warn(clippy::all)]

pub mod range;
pub mod response;

pub use range::{ByteRangeSpec, RangeError, SatisfiableRange, parse_range_header};
pub use response::{
    ContentRange, MediaHeadersRef, MediaResponsePlan, Method, ResponseStatus, plan_media_response,
};

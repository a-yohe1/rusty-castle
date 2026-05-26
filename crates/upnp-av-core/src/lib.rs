//! Sans-IO, `no_std` UPnP AV helpers.
#![no_std]
#![deny(unsafe_code)]
#![warn(clippy::all)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod connection_manager;
pub mod content_directory;
pub mod didl;

pub use content_directory::{BrowseFlag, BrowseRequestRef, BrowseResponseRef};

//! Sans-IO, `no_std` DLNA helpers.
#![no_std]
#![deny(unsafe_code)]
#![warn(clippy::all)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod protocol_info;

#[cfg(feature = "alloc")]
pub use protocol_info::content_features_to_string;
pub use protocol_info::{
    DlnaFlags, DlnaOp, DlnaProfile, ProtocolInfoRef, write_content_features, write_protocol_info,
};

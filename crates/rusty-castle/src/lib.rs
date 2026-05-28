//! Application-level MediaServer composition for rusty-castle.

pub mod catalog;
pub mod control;
pub mod description;
pub mod runtime;
pub mod scenario;

pub use catalog::{MediaContainer, MediaItem, StaticCatalog};
pub use control::{ControlError, ControlResponse, handle_control};
pub use description::{
    ServerConfig, connection_manager_scpd_xml, content_directory_scpd_xml, device_xml,
};

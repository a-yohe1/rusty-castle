//! Borrowed UPnP identifier types.

use core::fmt;

/// A borrowed UPnP device type, encoded as `urn:<domain>:device:<kind>:<version>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceTypeRef<'a> {
    /// Domain component, commonly `schemas-upnp-org`.
    pub domain: &'a str,
    /// Device kind, for example `MediaServer`.
    pub kind: &'a str,
    /// Device type version.
    pub version: u32,
}

impl fmt::Display for DeviceTypeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "urn:{}:device:{}:{}",
            self.domain, self.kind, self.version
        )
    }
}

/// A borrowed UPnP service type, encoded as `urn:<domain>:service:<kind>:<version>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceTypeRef<'a> {
    /// Domain component, commonly `schemas-upnp-org`.
    pub domain: &'a str,
    /// Service kind, for example `ContentDirectory`.
    pub kind: &'a str,
    /// Service type version.
    pub version: u32,
}

impl fmt::Display for ServiceTypeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "urn:{}:service:{}:{}",
            self.domain, self.kind, self.version
        )
    }
}

/// A borrowed UPnP Unique Device Name without the `uuid:` prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UdnRef<'a> {
    /// UUID text.
    pub uuid: &'a str,
}

impl fmt::Display for UdnRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "uuid:{}", self.uuid)
    }
}

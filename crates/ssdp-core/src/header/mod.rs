//! SSDP-defined HTTP header names and typed value parsers.

pub mod bootid;
pub mod cache_control;
pub mod man;
pub mod mx;
pub mod nts;
pub mod server;
pub mod target;
pub mod usn;

/// Canonical SSDP/HTTP header names (ASCII lowercase for comparison).
pub(crate) mod name {
    pub const HOST: &str = "host";
    pub const CACHE_CONTROL: &str = "cache-control";
    pub const LOCATION: &str = "location";
    pub const NT: &str = "nt";
    pub const NTS: &str = "nts";
    pub const SERVER: &str = "server";
    pub const ST: &str = "st";
    pub const USN: &str = "usn";
    pub const MAN: &str = "man";
    pub const MX: &str = "mx";
    pub const USER_AGENT: &str = "user-agent";
    pub const BOOTID: &str = "bootid.upnp.org";
    pub const CONFIGID: &str = "configid.upnp.org";
    pub const NEXTBOOTID: &str = "nextbootid.upnp.org";
    pub const SEARCHPORT: &str = "searchport.upnp.org";
    pub const CPFN: &str = "cpfn.upnp.org";
    pub const CPUUID: &str = "cpuuid.upnp.org";
    pub const TCPPORT: &str = "tcpport.upnp.org";
}

/// Case-insensitive equality check for header names.
#[inline]
pub(crate) fn header_eq(a: &[u8], canonical_lower: &str) -> bool {
    if a.len() != canonical_lower.len() {
        return false;
    }
    a.iter()
        .zip(canonical_lower.bytes())
        .all(|(&x, y)| x.to_ascii_lowercase() == y)
}

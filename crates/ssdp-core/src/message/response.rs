//! HTTP/1.1 200 OK response to M-SEARCH.

use crate::header::target::TargetRef;
use crate::header::usn::UsnRef;
use core::time::Duration;

/// A parsed `HTTP/1.1 200 OK` M-SEARCH response, borrowing from the datagram buffer.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchResponseRef<'a> {
    /// ST (Search Target that matched).
    pub st: TargetRef<'a>,
    /// USN (Unique Service Name).
    pub usn: UsnRef<'a>,
    /// URL of the device description.
    pub location: &'a str,
    /// Max-age from CACHE-CONTROL.
    pub max_age: Duration,
    /// SERVER header value.
    pub server: Option<&'a str>,
    /// `BOOTID.UPNP.ORG` (UPnP DA 1.1).
    pub bootid: Option<u32>,
    /// `CONFIGID.UPNP.ORG` (UPnP DA 1.1).
    pub configid: Option<u32>,
    /// `SEARCHPORT.UPNP.ORG` (UPnP DA 1.1).
    pub searchport: Option<u16>,
}

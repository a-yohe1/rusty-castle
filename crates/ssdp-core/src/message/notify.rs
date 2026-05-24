//! NOTIFY message type.

use crate::header::nts::Nts;
use crate::header::target::TargetRef;
use crate::header::usn::UsnRef;
use core::time::Duration;

/// Sub-type of a NOTIFY message, determined by the NTS header.
pub type NotifySubType = Nts;

/// A parsed `NOTIFY * HTTP/1.1` message, borrowing from the datagram buffer.
#[derive(Clone, Debug, PartialEq)]
pub struct NotifyRef<'a> {
    /// HOST header value.
    pub host: &'a str,
    /// NT (Notification Type) header value.
    pub nt: TargetRef<'a>,
    /// NTS (Notification Sub Type): alive, byebye, or update.
    pub nts: Nts,
    /// USN (Unique Service Name).
    pub usn: UsnRef<'a>,
    /// LOCATION header value (URL to the device description).
    /// Required for `ssdp:alive` and `ssdp:update`; absent for `ssdp:byebye`.
    pub location: Option<&'a str>,
    /// Max-age from CACHE-CONTROL; present for `ssdp:alive` and `ssdp:update`.
    pub max_age: Option<Duration>,
    /// SERVER header value.
    pub server: Option<&'a str>,
    /// `BOOTID.UPNP.ORG` (UPnP DA 1.1).
    pub bootid: Option<u32>,
    /// `CONFIGID.UPNP.ORG` (UPnP DA 1.1).
    pub configid: Option<u32>,
    /// `NEXTBOOTID.UPNP.ORG` — set only in `ssdp:update` (UPnP DA 1.1).
    pub nextbootid: Option<u32>,
    /// `SEARCHPORT.UPNP.ORG` — alternate port for unicast M-SEARCH responses.
    pub searchport: Option<u16>,
}

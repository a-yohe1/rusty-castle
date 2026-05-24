//! M-SEARCH request message type.

use crate::header::target::TargetRef;

/// A parsed `M-SEARCH * HTTP/1.1` request, borrowing from the datagram buffer.
#[derive(Clone, Debug, PartialEq)]
pub struct MSearchRef<'a> {
    /// HOST header value (e.g. `239.255.255.250:1900`).
    pub host: &'a str,
    /// ST (Search Target) header value.
    pub st: TargetRef<'a>,
    /// MX header value, clamped to 1–5.
    pub mx: u8,
    /// Optional USER-AGENT header.
    pub user_agent: Option<&'a str>,
    /// `CPFN.UPNP.ORG` — control point friendly name (UPnP DA 1.1).
    pub cpfn: Option<&'a str>,
    /// `CPUUID.UPNP.ORG` — control point UUID (UPnP DA 1.1).
    pub cpuuid: Option<&'a str>,
    /// `TCPPORT.UPNP.ORG` — TCP port for unicast search response (UPnP DA 1.1).
    pub tcpport: Option<u16>,
}

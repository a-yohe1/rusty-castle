//! SSDP message types.

pub mod msearch;
pub mod notify;
pub mod response;

pub use msearch::MSearchRef;
pub use notify::{NotifyRef, NotifySubType};
pub use response::SearchResponseRef;

/// A parsed SSDP datagram, borrowing from the input buffer.
#[derive(Clone, Debug, PartialEq)]
pub enum MessageRef<'a> {
    /// An `M-SEARCH * HTTP/1.1` request from a control point.
    Search(MSearchRef<'a>),
    /// An `HTTP/1.1 200 OK` response to an M-SEARCH.
    SearchResponse(SearchResponseRef<'a>),
    /// A `NOTIFY * HTTP/1.1` advertisement from a device.
    Notify(NotifyRef<'a>),
}

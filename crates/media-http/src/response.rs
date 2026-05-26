//! Media response planning.

use crate::range::{RangeError, SatisfiableRange, parse_range_header};
use dlna_core::ProtocolInfoRef;

/// Supported HTTP methods for media resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// GET request.
    Get,
    /// HEAD request.
    Head,
}

/// Planned HTTP status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseStatus {
    /// `200 OK`.
    Ok,
    /// `206 Partial Content`.
    PartialContent,
    /// `416 Range Not Satisfiable`.
    RangeNotSatisfiable,
}

/// Borrowed media metadata needed to construct DLNA-friendly headers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaHeadersRef<'a> {
    /// Total representation length in bytes.
    pub len: u64,
    /// MIME content type.
    pub content_type: &'a str,
    /// DLNA/UPnP protocol info for this resource.
    pub protocol_info: ProtocolInfoRef<'a>,
}

/// A media response plan independent of any concrete HTTP implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaResponsePlan<'a> {
    /// HTTP status.
    pub status: ResponseStatus,
    /// Whether a response body should be sent.
    pub send_body: bool,
    /// Content-Type header.
    pub content_type: &'a str,
    /// Content-Length header value.
    pub content_length: u64,
    /// Content-Range header value for partial or unsatisfiable responses.
    pub content_range: Option<ContentRange>,
    /// Inclusive file byte span to read for the body.
    pub body_range: Option<SatisfiableRange>,
    /// `Accept-Ranges` header value.
    pub accept_ranges: &'static str,
    /// `transferMode.dlna.org` header value.
    pub transfer_mode: &'static str,
    /// DLNA/UPnP protocol info for `contentFeatures.dlna.org`.
    pub protocol_info: ProtocolInfoRef<'a>,
}

/// Content-Range representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentRange {
    /// `bytes start-end/len`.
    Bytes {
        /// First byte position.
        start: u64,
        /// Last byte position.
        end: u64,
        /// Total representation length.
        complete_len: u64,
    },
    /// `bytes */len`.
    Unsatisfied {
        /// Total representation length.
        complete_len: u64,
    },
}

/// Plans a GET or HEAD response for a media resource.
pub fn plan_media_response<'a>(
    method: Method,
    headers: MediaHeadersRef<'a>,
    range_header: Option<&str>,
) -> MediaResponsePlan<'a> {
    let send_body = method == Method::Get;
    if let Some(range_header) = range_header {
        match parse_range_header(range_header).and_then(|range| range.apply(headers.len)) {
            Ok(range) => {
                return MediaResponsePlan {
                    status: ResponseStatus::PartialContent,
                    send_body,
                    content_type: headers.content_type,
                    content_length: range.len(),
                    content_range: Some(ContentRange::Bytes {
                        start: range.start,
                        end: range.end,
                        complete_len: headers.len,
                    }),
                    body_range: Some(range),
                    accept_ranges: "bytes",
                    transfer_mode: "Streaming",
                    protocol_info: headers.protocol_info,
                };
            }
            Err(RangeError::Unsatisfiable) => {
                return MediaResponsePlan {
                    status: ResponseStatus::RangeNotSatisfiable,
                    send_body: false,
                    content_type: headers.content_type,
                    content_length: 0,
                    content_range: Some(ContentRange::Unsatisfied {
                        complete_len: headers.len,
                    }),
                    body_range: None,
                    accept_ranges: "bytes",
                    transfer_mode: "Streaming",
                    protocol_info: headers.protocol_info,
                };
            }
            Err(RangeError::Invalid) => {}
        }
    }

    MediaResponsePlan {
        status: ResponseStatus::Ok,
        send_body,
        content_type: headers.content_type,
        content_length: headers.len,
        content_range: None,
        body_range: (headers.len != 0).then(|| SatisfiableRange {
            start: 0,
            end: headers.len - 1,
        }),
        accept_ranges: "bytes",
        transfer_mode: "Streaming",
        protocol_info: headers.protocol_info,
    }
}

//! Error types for parsing and encoding SSDP messages.

use core::fmt;

/// Errors that occur when parsing a raw SSDP datagram.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// The datagram is empty or too short to contain a valid SSDP message.
    TooShort,
    /// The start line is not a recognised SSDP request or response.
    UnknownStartLine,
    /// A header line does not end with CRLF.
    InvalidLineEnding,
    /// The underlying HTTP-level header parse failed.
    MalformedHeaders,
    /// A required header is absent.
    MissingHeader(&'static str),
    /// A header value could not be interpreted as the expected type.
    InvalidHeaderValue(&'static str),
    /// The MX value is outside the allowed range 1–5.
    MxOutOfRange,
    /// The NTS value is not one of `ssdp:alive`, `ssdp:byebye`, `ssdp:update`.
    UnknownNts,
    /// The NT/ST value could not be parsed as a valid SSDP target.
    InvalidTarget,
    /// The USN value does not start with `uuid:`.
    InvalidUsn,
    /// The CACHE-CONTROL header does not contain a valid `max-age` directive.
    InvalidCacheControl,
    /// Too many headers; increase the `MAX_HEADERS` constant.
    TooManyHeaders,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => f.write_str("datagram too short"),
            Self::UnknownStartLine => f.write_str("unrecognised SSDP start line"),
            Self::InvalidLineEnding => f.write_str("header line missing CRLF"),
            Self::MalformedHeaders => f.write_str("malformed HTTP headers"),
            Self::MissingHeader(n) => write!(f, "missing required header: {n}"),
            Self::InvalidHeaderValue(n) => write!(f, "invalid value for header: {n}"),
            Self::MxOutOfRange => f.write_str("MX value out of range 1-5"),
            Self::UnknownNts => f.write_str("unknown NTS value"),
            Self::InvalidTarget => f.write_str("invalid NT/ST target value"),
            Self::InvalidUsn => f.write_str("invalid USN value"),
            Self::InvalidCacheControl => f.write_str("invalid CACHE-CONTROL max-age"),
            Self::TooManyHeaders => f.write_str("too many headers in datagram"),
        }
    }
}

/// Errors that occur when encoding an SSDP message into a byte buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodeError {
    /// The provided output buffer is too small to hold the encoded message.
    BufferTooSmall,
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall => f.write_str("output buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}

#[cfg(feature = "std")]
impl std::error::Error for EncodeError {}

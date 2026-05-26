//! Error types for UPnP parsing and writing.

use core::fmt;

/// Errors produced while parsing UPnP XML payloads.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// The XML document did not contain a SOAP body.
    MissingBody,
    /// No SOAP action element was found inside the body.
    MissingAction,
    /// A required XML namespace declaration is absent.
    MissingNamespace,
    /// A type URN was not in the expected UPnP form.
    InvalidUrn,
    /// The XML structure is malformed or unsupported by the small parser.
    MalformedXml,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBody => f.write_str("missing SOAP body"),
            Self::MissingAction => f.write_str("missing SOAP action"),
            Self::MissingNamespace => f.write_str("missing SOAP action namespace"),
            Self::InvalidUrn => f.write_str("invalid UPnP URN"),
            Self::MalformedXml => f.write_str("malformed XML"),
        }
    }
}

/// Errors produced while writing UPnP XML.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WriteError {
    /// The target writer rejected the write.
    Fmt,
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fmt => f.write_str("XML writer failed"),
        }
    }
}

impl From<fmt::Error> for WriteError {
    fn from(_: fmt::Error) -> Self {
        Self::Fmt
    }
}

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}

#[cfg(feature = "std")]
impl std::error::Error for WriteError {}

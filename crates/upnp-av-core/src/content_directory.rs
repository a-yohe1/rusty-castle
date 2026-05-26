//! ContentDirectory action model and SOAP response helpers.

use core::fmt::Write;
use upnp_core::ids::ServiceTypeRef;
use upnp_core::soap::{ResponseArgumentRef, write_action_response};

/// ContentDirectory service type.
pub const CONTENT_DIRECTORY_SERVICE: ServiceTypeRef<'static> = ServiceTypeRef {
    domain: "schemas-upnp-org",
    kind: "ContentDirectory",
    version: 1,
};

/// Browse flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowseFlag {
    /// Return metadata for the requested object.
    Metadata,
    /// Return direct children of the requested container.
    DirectChildren,
}

impl BrowseFlag {
    /// Parses a UPnP BrowseFlag value.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "BrowseMetadata" => Some(Self::Metadata),
            "BrowseDirectChildren" => Some(Self::DirectChildren),
            _ => None,
        }
    }
}

/// Borrowed ContentDirectory Browse request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowseRequestRef<'a> {
    /// Object id.
    pub object_id: &'a str,
    /// Browse flag.
    pub flag: BrowseFlag,
    /// Filter string.
    pub filter: &'a str,
    /// Starting index.
    pub starting_index: u32,
    /// Requested count.
    pub requested_count: u32,
    /// Sort criteria string.
    pub sort_criteria: &'a str,
}

/// Borrowed ContentDirectory Browse response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowseResponseRef<'a> {
    /// DIDL-Lite result XML.
    pub result: &'a str,
    /// Number returned.
    pub number_returned: u32,
    /// Total matches.
    pub total_matches: u32,
    /// Update id.
    pub update_id: u32,
}

/// Writes a SOAP Browse response.
pub fn write_browse_response<W: Write + ?Sized>(
    w: &mut W,
    response: &BrowseResponseRef<'_>,
) -> Result<(), upnp_core::WriteError> {
    let number_returned = U32Display::new(response.number_returned);
    let total_matches = U32Display::new(response.total_matches);
    let update_id = U32Display::new(response.update_id);
    let number_returned = number_returned.as_str();
    let total_matches = total_matches.as_str();
    let update_id = update_id.as_str();
    write_action_response(
        w,
        &CONTENT_DIRECTORY_SERVICE,
        "Browse",
        &[
            ResponseArgumentRef {
                name: "Result",
                value: response.result,
            },
            ResponseArgumentRef {
                name: "NumberReturned",
                value: number_returned,
            },
            ResponseArgumentRef {
                name: "TotalMatches",
                value: total_matches,
            },
            ResponseArgumentRef {
                name: "UpdateID",
                value: update_id,
            },
        ],
    )
}

/// Writes a SOAP GetSystemUpdateID response.
pub fn write_system_update_id_response<W: Write + ?Sized>(
    w: &mut W,
    update_id: u32,
) -> Result<(), upnp_core::WriteError> {
    let update_id = U32Display::new(update_id);
    let update_id = update_id.as_str();
    write_action_response(
        w,
        &CONTENT_DIRECTORY_SERVICE,
        "GetSystemUpdateID",
        &[ResponseArgumentRef {
            name: "Id",
            value: update_id,
        }],
    )
}

struct U32Display {
    buf: [u8; 10],
    start: usize,
}

impl U32Display {
    fn new(value: u32) -> Self {
        let mut this = Self {
            buf: [0; 10],
            start: 10,
        };
        if value == 0 {
            this.start = 9;
            this.buf[9] = b'0';
            return this;
        }
        let mut value = value;
        while value > 0 {
            this.start -= 1;
            this.buf[this.start] = b'0' + (value % 10) as u8;
            value /= 10;
        }
        this
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[self.start..]).unwrap_or("0")
    }
}

impl From<u32> for U32Display {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

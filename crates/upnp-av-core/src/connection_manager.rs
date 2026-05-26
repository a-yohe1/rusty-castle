//! ConnectionManager response helpers.

use core::fmt::{self, Write};
use dlna_core::ProtocolInfoRef;
use upnp_core::ids::ServiceTypeRef;
use upnp_core::soap::{ResponseArgumentRef, write_action_response};

/// ConnectionManager service type.
pub const CONNECTION_MANAGER_SERVICE: ServiceTypeRef<'static> = ServiceTypeRef {
    domain: "schemas-upnp-org",
    kind: "ConnectionManager",
    version: 1,
};

/// Writes a comma-separated protocolInfo list.
pub fn write_protocol_info_list<W: Write + ?Sized>(
    w: &mut W,
    values: &[ProtocolInfoRef<'_>],
) -> fmt::Result {
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            w.write_char(',')?;
        }
        write!(w, "{value}")?;
    }
    Ok(())
}

/// Builds a comma-separated protocolInfo list in an owned string.
#[cfg(feature = "alloc")]
pub fn protocol_info_list_to_string(
    values: &[ProtocolInfoRef<'_>],
) -> Result<alloc::string::String, fmt::Error> {
    let mut out = alloc::string::String::new();
    write_protocol_info_list(&mut out, values)?;
    Ok(out)
}

/// Writes a SOAP GetProtocolInfo response.
pub fn write_get_protocol_info_response<W: Write + ?Sized>(
    w: &mut W,
    sink: &str,
    source: &str,
) -> Result<(), upnp_core::WriteError> {
    write_action_response(
        w,
        &CONNECTION_MANAGER_SERVICE,
        "GetProtocolInfo",
        &[
            ResponseArgumentRef {
                name: "Source",
                value: source,
            },
            ResponseArgumentRef {
                name: "Sink",
                value: sink,
            },
        ],
    )
}

/// Writes a SOAP GetCurrentConnectionIDs response with no active connections.
pub fn write_current_connection_ids_response<W: Write + ?Sized>(
    w: &mut W,
) -> Result<(), upnp_core::WriteError> {
    write_action_response(
        w,
        &CONNECTION_MANAGER_SERVICE,
        "GetCurrentConnectionIDs",
        &[ResponseArgumentRef {
            name: "ConnectionIDs",
            value: "0",
        }],
    )
}

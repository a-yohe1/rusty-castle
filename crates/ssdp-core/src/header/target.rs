//! NT / ST header values: SSDP search/notification target types.

use crate::error::ParseError;

/// A parsed SSDP search target (ST) or notification type (NT), borrowing from the
/// original datagram buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetRef<'a> {
    /// `ssdp:all` — match all UPnP devices and services.
    All,
    /// `upnp:rootdevice` — root device advertisements only.
    RootDevice,
    /// `uuid:<device-UUID>` — target a specific device instance.
    Uuid(&'a str),
    /// `urn:<domain>:device:<type>:<version>` — all devices of a given type.
    DeviceType {
        /// Domain (e.g. `schemas-upnp-org`).
        domain: &'a str,
        /// Device type name.
        kind: &'a str,
        /// Version number.
        version: u32,
    },
    /// `urn:<domain>:service:<type>:<version>` — all services of a given type.
    ServiceType {
        /// Domain.
        domain: &'a str,
        /// Service type name.
        kind: &'a str,
        /// Version number.
        version: u32,
    },
}

impl<'a> TargetRef<'a> {
    /// Parses a raw NT or ST header value.
    pub fn parse(value: &'a str) -> Result<Self, ParseError> {
        let v = value.trim();
        if v.eq_ignore_ascii_case("ssdp:all") {
            return Ok(Self::All);
        }
        if v.eq_ignore_ascii_case("upnp:rootdevice") {
            return Ok(Self::RootDevice);
        }
        if let Some(uuid) = v.strip_prefix("uuid:") {
            if uuid.is_empty() {
                return Err(ParseError::InvalidTarget);
            }
            return Ok(Self::Uuid(uuid));
        }
        if let Some(rest) = v.strip_prefix("urn:") {
            return parse_urn(rest);
        }
        Err(ParseError::InvalidTarget)
    }
}

fn parse_urn(rest: &str) -> Result<TargetRef<'_>, ParseError> {
    // expected: <domain>:<device|service>:<type>:<version>
    let mut parts = rest.splitn(4, ':');
    let domain = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(ParseError::InvalidTarget)?;
    let kind_str = parts.next().ok_or(ParseError::InvalidTarget)?;
    let type_name = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(ParseError::InvalidTarget)?;
    let ver_str = parts.next().ok_or(ParseError::InvalidTarget)?;
    let version = ver_str
        .parse::<u32>()
        .map_err(|_| ParseError::InvalidTarget)?;

    match kind_str {
        "device" => Ok(TargetRef::DeviceType {
            domain,
            kind: type_name,
            version,
        }),
        "service" => Ok(TargetRef::ServiceType {
            domain,
            kind: type_name,
            version,
        }),
        _ => Err(ParseError::InvalidTarget),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all() {
        assert_eq!(TargetRef::parse("ssdp:all"), Ok(TargetRef::All));
    }

    #[test]
    fn root_device() {
        assert_eq!(
            TargetRef::parse("upnp:rootdevice"),
            Ok(TargetRef::RootDevice)
        );
    }

    #[test]
    fn uuid() {
        assert_eq!(
            TargetRef::parse("uuid:550e8400-e29b-41d4-a716-446655440000"),
            Ok(TargetRef::Uuid("550e8400-e29b-41d4-a716-446655440000"))
        );
    }

    #[test]
    fn device_type() {
        assert_eq!(
            TargetRef::parse("urn:schemas-upnp-org:device:MediaServer:1"),
            Ok(TargetRef::DeviceType {
                domain: "schemas-upnp-org",
                kind: "MediaServer",
                version: 1,
            })
        );
    }

    #[test]
    fn service_type() {
        assert_eq!(
            TargetRef::parse("urn:schemas-upnp-org:service:ContentDirectory:1"),
            Ok(TargetRef::ServiceType {
                domain: "schemas-upnp-org",
                kind: "ContentDirectory",
                version: 1,
            })
        );
    }

    #[test]
    fn invalid_urn_no_version() {
        assert_eq!(
            TargetRef::parse("urn:schemas-upnp-org:device:MediaServer"),
            Err(ParseError::InvalidTarget)
        );
    }
}

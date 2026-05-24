//! USN (Unique Service Name) header parsing.
//!
//! USN format (UPnP DA 1.1):
//! - `uuid:<device-UUID>`
//! - `uuid:<device-UUID>::upnp:rootdevice`
//! - `uuid:<device-UUID>::urn:<domain>:device:<type>:<ver>`
//! - `uuid:<device-UUID>::urn:<domain>:service:<type>:<ver>`

use crate::error::ParseError;
use crate::header::target::TargetRef;

/// A parsed USN header value, borrowing from the original buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsnRef<'a> {
    /// The device UUID (the part after `uuid:` before `::`, if any).
    pub device_uuid: &'a str,
    /// Additional NT/ST qualifier after `::`, if present.
    pub embedded: Option<TargetRef<'a>>,
}

impl<'a> UsnRef<'a> {
    /// Parses a raw USN header value.
    pub fn parse(value: &'a str) -> Result<Self, ParseError> {
        let v = value.trim();
        let after_uuid = v.strip_prefix("uuid:").ok_or(ParseError::InvalidUsn)?;

        match after_uuid.find("::") {
            None => {
                if after_uuid.is_empty() {
                    return Err(ParseError::InvalidUsn);
                }
                Ok(Self {
                    device_uuid: after_uuid,
                    embedded: None,
                })
            }
            Some(sep) => {
                let uuid = &after_uuid[..sep];
                let rest = &after_uuid[sep + 2..];
                if uuid.is_empty() || rest.is_empty() {
                    return Err(ParseError::InvalidUsn);
                }
                let embedded = TargetRef::parse(rest)?;
                Ok(Self {
                    device_uuid: uuid,
                    embedded: Some(embedded),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_only() {
        let usn = UsnRef::parse("uuid:550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(usn.device_uuid, "550e8400-e29b-41d4-a716-446655440000");
        assert!(usn.embedded.is_none());
    }

    #[test]
    fn root_device() {
        let usn =
            UsnRef::parse("uuid:550e8400-e29b-41d4-a716-446655440000::upnp:rootdevice").unwrap();
        assert_eq!(usn.device_uuid, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(usn.embedded, Some(TargetRef::RootDevice));
    }

    #[test]
    fn device_type() {
        let usn = UsnRef::parse(
            "uuid:550e8400-e29b-41d4-a716-446655440000::urn:schemas-upnp-org:device:MediaServer:1",
        )
        .unwrap();
        assert_eq!(
            usn.embedded,
            Some(TargetRef::DeviceType {
                domain: "schemas-upnp-org",
                kind: "MediaServer",
                version: 1
            })
        );
    }

    #[test]
    fn missing_uuid_prefix() {
        assert_eq!(
            UsnRef::parse("550e8400::upnp:rootdevice"),
            Err(ParseError::InvalidUsn)
        );
    }
}

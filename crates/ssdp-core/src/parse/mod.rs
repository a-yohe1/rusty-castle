//! SSDP datagram parser.

mod headers;
mod start_line;

use crate::error::ParseError;
use crate::message::MessageRef;
use start_line::StartLine;

/// Parses a raw UDP datagram as an SSDP message.
///
/// Returns a [`MessageRef`] that borrows directly from `buf` — no allocation is performed.
///
/// # Errors
///
/// Returns [`ParseError`] if the datagram is malformed, has missing required headers, or
/// contains invalid header values.
pub fn parse_datagram(buf: &[u8]) -> Result<MessageRef<'_>, ParseError> {
    if buf.is_empty() {
        return Err(ParseError::TooShort);
    }

    let (kind, rest) = start_line::parse(buf)?;

    match kind {
        StartLine::MSearch => {
            let msg = headers::parse_msearch(rest)?;
            Ok(MessageRef::Search(msg))
        }
        StartLine::Notify => {
            let msg = headers::parse_notify(rest)?;
            Ok(MessageRef::Notify(msg))
        }
        StartLine::Response200 => {
            let msg = headers::parse_response(rest)?;
            Ok(MessageRef::SearchResponse(msg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::nts::Nts;
    use crate::header::target::TargetRef;

    #[test]
    fn parse_msearch_basic() {
        let pkt = b"M-SEARCH * HTTP/1.1\r\n\
                    HOST: 239.255.255.250:1900\r\n\
                    MAN: \"ssdp:discover\"\r\n\
                    MX: 3\r\n\
                    ST: ssdp:all\r\n\
                    \r\n";
        let msg = parse_datagram(pkt).unwrap();
        let MessageRef::Search(m) = msg else {
            panic!("expected Search")
        };
        assert_eq!(m.mx, 3);
        assert_eq!(m.st, TargetRef::All);
    }

    #[test]
    fn parse_notify_alive() {
        let pkt = b"NOTIFY * HTTP/1.1\r\n\
                    HOST: 239.255.255.250:1900\r\n\
                    NT: upnp:rootdevice\r\n\
                    NTS: ssdp:alive\r\n\
                    USN: uuid:550e8400-e29b-41d4-a716-446655440000::upnp:rootdevice\r\n\
                    LOCATION: http://192.168.1.1:80/desc.xml\r\n\
                    CACHE-CONTROL: max-age=1800\r\n\
                    SERVER: Linux/5.4 UPnP/1.1 TestDevice/1.0\r\n\
                    \r\n";
        let msg = parse_datagram(pkt).unwrap();
        let MessageRef::Notify(n) = msg else {
            panic!("expected Notify")
        };
        assert_eq!(n.nts, Nts::Alive);
        assert_eq!(n.location, Some("http://192.168.1.1:80/desc.xml"));
    }

    #[test]
    fn parse_notify_byebye_no_location() {
        let pkt = b"NOTIFY * HTTP/1.1\r\n\
                    HOST: 239.255.255.250:1900\r\n\
                    NT: upnp:rootdevice\r\n\
                    NTS: ssdp:byebye\r\n\
                    USN: uuid:550e8400-e29b-41d4-a716-446655440000::upnp:rootdevice\r\n\
                    \r\n";
        let msg = parse_datagram(pkt).unwrap();
        let MessageRef::Notify(n) = msg else {
            panic!("expected Notify")
        };
        assert_eq!(n.nts, Nts::ByeBye);
        assert!(n.location.is_none());
    }

    #[test]
    fn empty_datagram() {
        assert_eq!(parse_datagram(b""), Err(ParseError::TooShort));
    }

    #[test]
    fn missing_required_header() {
        // M-SEARCH without ST
        let pkt = b"M-SEARCH * HTTP/1.1\r\n\
                    HOST: 239.255.255.250:1900\r\n\
                    MAN: \"ssdp:discover\"\r\n\
                    MX: 3\r\n\
                    \r\n";
        assert_eq!(parse_datagram(pkt), Err(ParseError::MissingHeader("ST")));
    }
}

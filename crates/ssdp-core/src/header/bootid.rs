//! UPnP DA 1.1 extension headers:
//! `BOOTID.UPNP.ORG`, `CONFIGID.UPNP.ORG`, `NEXTBOOTID.UPNP.ORG`, `SEARCHPORT.UPNP.ORG`.

use crate::error::ParseError;

/// Parses a non-negative decimal integer header value (used for BOOTID, CONFIGID,
/// NEXTBOOTID).
pub fn parse_u32(header_name: &'static str, value: &str) -> Result<u32, ParseError> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|_| ParseError::InvalidHeaderValue(header_name))
}

/// Parses a SEARCHPORT value (1–65535).
pub fn parse_searchport(value: &str) -> Result<u16, ParseError> {
    let port: u16 = value
        .trim()
        .parse()
        .map_err(|_| ParseError::InvalidHeaderValue("SEARCHPORT.UPNP.ORG"))?;
    if port == 0 {
        return Err(ParseError::InvalidHeaderValue("SEARCHPORT.UPNP.ORG"));
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootid() {
        assert_eq!(parse_u32("BOOTID.UPNP.ORG", "1"), Ok(1));
        assert_eq!(parse_u32("BOOTID.UPNP.ORG", "4294967295"), Ok(u32::MAX));
        assert!(parse_u32("BOOTID.UPNP.ORG", "abc").is_err());
    }

    #[test]
    fn searchport() {
        assert_eq!(parse_searchport("5000"), Ok(5000));
        assert!(parse_searchport("0").is_err());
        assert!(parse_searchport("99999").is_err());
    }
}

//! NTS (Notification Sub Type) header values.

use crate::error::ParseError;

/// Parsed NTS header value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nts {
    /// `ssdp:alive` — device or service is available.
    Alive,
    /// `ssdp:byebye` — device or service is about to become unavailable.
    ByeBye,
    /// `ssdp:update` — device has changed its BOOTID (UPnP DA 1.1).
    Update,
}

impl Nts {
    /// Parses an NTS header value.
    pub fn parse(value: &str) -> Result<Self, ParseError> {
        let v = value.trim();
        if v.eq_ignore_ascii_case("ssdp:alive") {
            Ok(Self::Alive)
        } else if v.eq_ignore_ascii_case("ssdp:byebye") {
            Ok(Self::ByeBye)
        } else if v.eq_ignore_ascii_case("ssdp:update") {
            Ok(Self::Update)
        } else {
            Err(ParseError::UnknownNts)
        }
    }

    /// Returns the canonical wire representation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Alive => "ssdp:alive",
            Self::ByeBye => "ssdp:byebye",
            Self::Update => "ssdp:update",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for nts in [Nts::Alive, Nts::ByeBye, Nts::Update] {
            assert_eq!(Nts::parse(nts.as_str()), Ok(nts));
        }
    }

    #[test]
    fn unknown() {
        assert_eq!(Nts::parse("ssdp:other"), Err(ParseError::UnknownNts));
    }
}

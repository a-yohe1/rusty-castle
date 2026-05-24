//! MAN header: must be `"ssdp:discover"` in M-SEARCH requests.

use crate::error::ParseError;

/// Expected MAN header value (with surrounding quotes as required by spec).
pub const EXPECTED: &str = "\"ssdp:discover\"";

/// Validates that the MAN header value equals `"ssdp:discover"`.
pub fn validate(value: &str) -> Result<(), ParseError> {
    if value.trim() == EXPECTED {
        Ok(())
    } else {
        Err(ParseError::InvalidHeaderValue("MAN"))
    }
}

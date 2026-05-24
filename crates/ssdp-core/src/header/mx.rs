//! MX header: maximum wait time (seconds) in M-SEARCH requests.
//!
//! Valid range is 1–5 seconds per UPnP DA 1.1 §1.3.2.

use crate::consts::{MX_MAX, MX_MIN};
use crate::error::ParseError;

/// Parses and validates an MX header value.
///
/// Returns `ParseError::MxOutOfRange` if the value is outside 1–5.
pub fn parse(value: &str) -> Result<u8, ParseError> {
    let n: u8 = value.trim().parse().map_err(|_| ParseError::MxOutOfRange)?;
    if !(MX_MIN..=MX_MAX).contains(&n) {
        return Err(ParseError::MxOutOfRange);
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid() {
        assert_eq!(parse("3"), Ok(3));
        assert_eq!(parse("1"), Ok(1));
        assert_eq!(parse("5"), Ok(5));
    }

    #[test]
    fn out_of_range() {
        assert_eq!(parse("0"), Err(ParseError::MxOutOfRange));
        assert_eq!(parse("6"), Err(ParseError::MxOutOfRange));
    }

    #[test]
    fn not_a_number() {
        assert_eq!(parse("abc"), Err(ParseError::MxOutOfRange));
    }
}

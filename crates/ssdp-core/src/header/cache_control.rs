//! CACHE-CONTROL header: `max-age=<seconds>` parsing and formatting.

use crate::error::ParseError;
use core::time::Duration;

/// Parses the `max-age=<N>` directive from a CACHE-CONTROL header value.
///
/// Ignores other directives. Returns `ParseError::InvalidCacheControl` if no
/// valid `max-age` directive is found or if the value overflows `u32`.
pub fn parse_max_age(value: &str) -> Result<Duration, ParseError> {
    for part in value.split(',') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("max-age") {
            let rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
            if let Some(num_str) = rest.strip_prefix('=') {
                let n = num_str
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| ParseError::InvalidCacheControl)?;
                return Ok(Duration::from_secs(u64::from(n)));
            }
        }
    }
    Err(ParseError::InvalidCacheControl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        assert_eq!(parse_max_age("max-age=1800"), Ok(Duration::from_secs(1800)));
    }

    #[test]
    fn with_extra_directives() {
        assert_eq!(
            parse_max_age("no-cache, max-age = 300"),
            Ok(Duration::from_secs(300))
        );
    }

    #[test]
    fn missing() {
        assert_eq!(
            parse_max_age("no-cache"),
            Err(ParseError::InvalidCacheControl)
        );
    }

    #[test]
    fn zero() {
        assert_eq!(parse_max_age("max-age=0"), Ok(Duration::ZERO));
    }
}

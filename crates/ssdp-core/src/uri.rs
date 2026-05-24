//! Minimal URI validator used for LOCATION header values.
//!
//! We do not fully parse URIs; we only reject obviously invalid values
//! (control characters, empty string, unreasonable length).

/// Maximum allowed length for a LOCATION URI (bytes).
pub const MAX_URI_LEN: usize = 2048;

/// Returns `true` if `s` is an acceptable LOCATION URI.
///
/// Accepts any non-empty ASCII string without control characters and within
/// [`MAX_URI_LEN`] bytes.
#[inline]
pub fn is_valid(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_URI_LEN && s.bytes().all(|b| (0x20..0x7F).contains(&b))
}

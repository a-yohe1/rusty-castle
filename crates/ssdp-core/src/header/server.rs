//! SERVER / USER-AGENT header helpers.
//!
//! Format (UPnP DA 1.1): `<OS>/<version> UPnP/1.1 <product>/<version>`

/// Maximum useful length for a SERVER header value.
pub const MAX_SERVER_LEN: usize = 256;

/// Validates that a SERVER value is non-empty printable ASCII within the length limit.
pub fn is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SERVER_LEN
        && value.bytes().all(|b| (0x20..0x7F).contains(&b))
}

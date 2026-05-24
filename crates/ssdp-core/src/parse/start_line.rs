//! SSDP start-line classifier.

use crate::error::ParseError;

/// The kind of SSDP message identified from the start line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartLine {
    /// `M-SEARCH * HTTP/1.1`
    MSearch,
    /// `NOTIFY * HTTP/1.1`
    Notify,
    /// `HTTP/1.1 200 OK`
    Response200,
}

/// Parses the first line of a datagram and returns the message kind and the remaining bytes.
pub(crate) fn parse(buf: &[u8]) -> Result<(StartLine, &[u8]), ParseError> {
    let crlf = find_crlf(buf).ok_or(ParseError::TooShort)?;
    let line = core::str::from_utf8(&buf[..crlf]).map_err(|_| ParseError::UnknownStartLine)?;
    let rest = &buf[crlf + 2..];

    let kind = if line.eq_ignore_ascii_case("M-SEARCH * HTTP/1.1") {
        StartLine::MSearch
    } else if line.eq_ignore_ascii_case("NOTIFY * HTTP/1.1") {
        StartLine::Notify
    } else if line.starts_with("HTTP/1.1 200") {
        StartLine::Response200
    } else {
        return Err(ParseError::UnknownStartLine);
    };

    Ok((kind, rest))
}

fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msearch() {
        let (kind, _) = parse(b"M-SEARCH * HTTP/1.1\r\nHost: ...\r\n\r\n").unwrap();
        assert_eq!(kind, StartLine::MSearch);
    }

    #[test]
    fn notify() {
        let (kind, _) = parse(b"NOTIFY * HTTP/1.1\r\nHost: ...\r\n\r\n").unwrap();
        assert_eq!(kind, StartLine::Notify);
    }

    #[test]
    fn response() {
        let (kind, _) = parse(b"HTTP/1.1 200 OK\r\nST: ssdp:all\r\n\r\n").unwrap();
        assert_eq!(kind, StartLine::Response200);
    }

    #[test]
    fn unknown() {
        assert_eq!(
            parse(b"GET / HTTP/1.1\r\n\r\n"),
            Err(ParseError::UnknownStartLine)
        );
    }

    #[test]
    fn no_crlf() {
        assert_eq!(parse(b"M-SEARCH * HTTP/1.1"), Err(ParseError::TooShort));
    }
}

//! SSDP message encoders.

pub mod msearch;
pub mod notify;
pub mod response;

pub use msearch::encode_msearch;
pub use notify::encode_notify;
pub use response::encode_response;

use crate::error::EncodeError;
use core::fmt::Write as FmtWrite;

/// A write cursor over a mutable byte buffer.
///
/// Tracks the current write position and returns `EncodeError::BufferTooSmall` if
/// the buffer is exhausted.
pub(crate) struct BufWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> BufWriter<'a> {
    pub(crate) fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Returns the number of bytes written.
    pub(crate) fn written(&self) -> usize {
        self.pos
    }

    pub(crate) fn write_bytes(&mut self, src: &[u8]) -> Result<(), EncodeError> {
        let end = self.pos + src.len();
        if end > self.buf.len() {
            return Err(EncodeError::BufferTooSmall);
        }
        self.buf[self.pos..end].copy_from_slice(src);
        self.pos = end;
        Ok(())
    }

    pub(crate) fn write_str(&mut self, s: &str) -> Result<(), EncodeError> {
        self.write_bytes(s.as_bytes())
    }

    pub(crate) fn write_crlf(&mut self) -> Result<(), EncodeError> {
        self.write_bytes(b"\r\n")
    }

    /// Writes `name: value\r\n`.
    pub(crate) fn write_header(&mut self, name: &str, value: &str) -> Result<(), EncodeError> {
        self.write_str(name)?;
        self.write_bytes(b": ")?;
        self.write_str(value)?;
        self.write_crlf()
    }
}

impl FmtWrite for BufWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_bytes(s.as_bytes()).map_err(|_| core::fmt::Error)
    }
}

/// Writes a decimal u32 into the buffer using a small scratch buffer.
pub(crate) fn write_u32(w: &mut BufWriter<'_>, n: u32) -> Result<(), EncodeError> {
    let mut buf = [0u8; 10];
    let s = u32_to_str(n, &mut buf);
    w.write_bytes(s.as_bytes())
}

fn u32_to_str(n: u32, buf: &mut [u8; 10]) -> &str {
    if n == 0 {
        return "0";
    }
    let mut i = 10;
    let mut v = n;
    while v > 0 {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    core::str::from_utf8(&buf[i..]).unwrap()
}

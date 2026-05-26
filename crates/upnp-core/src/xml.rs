//! Minimal XML writing helpers.

use crate::error::WriteError;
use core::fmt::{self, Write};

/// Writes escaped XML character data.
pub fn write_escaped_text<W: Write + ?Sized>(w: &mut W, value: &str) -> Result<(), WriteError> {
    for ch in value.chars() {
        match ch {
            '&' => w.write_str("&amp;")?,
            '<' => w.write_str("&lt;")?,
            '>' => w.write_str("&gt;")?,
            '"' => w.write_str("&quot;")?,
            '\'' => w.write_str("&apos;")?,
            _ => w.write_char(ch)?,
        }
    }
    Ok(())
}

/// Writes `<name>escaped value</name>`.
pub fn write_text_element<W: Write + ?Sized>(
    w: &mut W,
    name: &str,
    value: &str,
) -> Result<(), WriteError> {
    write!(w, "<{name}>")?;
    write_escaped_text(w, value)?;
    write!(w, "</{name}>")?;
    Ok(())
}

/// Writes a typed value using its display implementation inside an XML element.
pub fn write_display_element<W, T>(w: &mut W, name: &str, value: &T) -> Result<(), WriteError>
where
    W: Write + ?Sized,
    T: fmt::Display + ?Sized,
{
    write!(w, "<{name}>{value}</{name}>")?;
    Ok(())
}

#[cfg(feature = "alloc")]
pub(crate) fn collect_to_string<F>(f: F) -> Result<alloc::string::String, WriteError>
where
    F: FnOnce(&mut alloc::string::String) -> Result<(), WriteError>,
{
    let mut out = alloc::string::String::new();
    f(&mut out)?;
    Ok(out)
}

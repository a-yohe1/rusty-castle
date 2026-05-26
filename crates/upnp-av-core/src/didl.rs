//! DIDL-Lite XML model and writer.

use core::fmt::{self, Write};
use dlna_core::ProtocolInfoRef;
use upnp_core::xml::write_escaped_text;

/// A DIDL-Lite object class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpnpClass {
    /// `object.container`.
    Container,
    /// `object.item.videoItem`.
    VideoItem,
    /// A custom class string.
    Custom(&'static str),
}

impl UpnpClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::Container => "object.container",
            Self::VideoItem => "object.item.videoItem",
            Self::Custom(value) => value,
        }
    }
}

/// A DIDL-Lite resource element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceRef<'a> {
    /// Resource URL.
    pub url: &'a str,
    /// UPnP/DLNA protocol info.
    pub protocol_info: ProtocolInfoRef<'a>,
    /// Optional byte size.
    pub size: Option<u64>,
    /// Optional duration in `H:MM:SS` or `H:MM:SS.mmm` form.
    pub duration: Option<&'a str>,
}

/// A DIDL-Lite object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectRef<'a> {
    /// Object id.
    pub id: &'a str,
    /// Parent object id.
    pub parent_id: &'a str,
    /// Whether the object is restricted.
    pub restricted: bool,
    /// Display title.
    pub title: &'a str,
    /// UPnP class.
    pub class: UpnpClass,
    /// Container child count.
    pub child_count: Option<u32>,
    /// Resource list for items.
    pub resources: &'a [ResourceRef<'a>],
}

/// Writes a complete DIDL-Lite document.
pub fn write_didl<W: Write + ?Sized>(
    w: &mut W,
    objects: &[ObjectRef<'_>],
) -> Result<(), fmt::Error> {
    w.write_str(r#"<DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" "#)?;
    w.write_str(r#"xmlns:dc="http://purl.org/dc/elements/1.1/" "#)?;
    w.write_str(r#"xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/">"#)?;
    for object in objects {
        write_object(w, object)?;
    }
    w.write_str("</DIDL-Lite>")?;
    Ok(())
}

/// Builds a DIDL-Lite document in an owned string.
#[cfg(feature = "alloc")]
pub fn didl_to_string(objects: &[ObjectRef<'_>]) -> Result<alloc::string::String, fmt::Error> {
    let mut out = alloc::string::String::new();
    write_didl(&mut out, objects)?;
    Ok(out)
}

fn write_object<W: Write + ?Sized>(w: &mut W, object: &ObjectRef<'_>) -> Result<(), fmt::Error> {
    let tag = match object.class {
        UpnpClass::Container => "container",
        _ => "item",
    };
    write!(
        w,
        r#"<{tag} id="{}" parentID="{}" restricted="{}""#,
        object.id,
        object.parent_id,
        if object.restricted { "1" } else { "0" }
    )?;
    if let Some(child_count) = object.child_count {
        write!(w, r#" childCount="{child_count}""#)?;
    }
    w.write_char('>')?;
    w.write_str("<dc:title>")?;
    write_escaped_text(w, object.title).map_err(|_| fmt::Error)?;
    w.write_str("</dc:title><upnp:class>")?;
    w.write_str(object.class.as_str())?;
    w.write_str("</upnp:class>")?;
    for res in object.resources {
        write_resource(w, res)?;
    }
    write!(w, "</{tag}>")?;
    Ok(())
}

fn write_resource<W: Write + ?Sized>(w: &mut W, res: &ResourceRef<'_>) -> Result<(), fmt::Error> {
    write!(w, r#"<res protocolInfo=""#)?;
    write!(w, "{}", res.protocol_info)?;
    w.write_char('"')?;
    if let Some(size) = res.size {
        write!(w, r#" size="{size}""#)?;
    }
    if let Some(duration) = res.duration {
        write!(w, r#" duration="{duration}""#)?;
    }
    w.write_char('>')?;
    write_escaped_text(w, res.url).map_err(|_| fmt::Error)?;
    w.write_str("</res>")?;
    Ok(())
}

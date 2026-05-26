//! Device description XML model and writer.

use crate::error::WriteError;
use crate::ids::{DeviceTypeRef, UdnRef};
#[cfg(feature = "alloc")]
use crate::xml::collect_to_string;
use crate::xml::{write_display_element, write_text_element};
use core::fmt::Write;

/// UPnP device architecture spec version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecVersion {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
}

impl Default for SpecVersion {
    fn default() -> Self {
        Self { major: 1, minor: 0 }
    }
}

/// A service entry inside a device description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceRef<'a> {
    /// Service type URN.
    pub service_type: crate::ids::ServiceTypeRef<'a>,
    /// Service id, for example `urn:upnp-org:serviceId:ContentDirectory`.
    pub service_id: &'a str,
    /// URL of the service description document.
    pub scpd_url: &'a str,
    /// SOAP control endpoint URL.
    pub control_url: &'a str,
    /// Event subscription URL.
    pub event_sub_url: &'a str,
}

/// Device icon metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IconRef<'a> {
    /// MIME type.
    pub mimetype: &'a str,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Color depth.
    pub depth: u32,
    /// Icon URL.
    pub url: &'a str,
}

/// A UPnP device description node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceRef<'a> {
    /// Device type URN.
    pub device_type: DeviceTypeRef<'a>,
    /// Human-readable device name.
    pub friendly_name: &'a str,
    /// Manufacturer name.
    pub manufacturer: &'a str,
    /// Optional manufacturer URL.
    pub manufacturer_url: Option<&'a str>,
    /// Optional model description.
    pub model_description: Option<&'a str>,
    /// Model name.
    pub model_name: &'a str,
    /// Optional model number.
    pub model_number: Option<&'a str>,
    /// Optional model URL.
    pub model_url: Option<&'a str>,
    /// Optional serial number.
    pub serial_number: Option<&'a str>,
    /// Unique device name.
    pub udn: UdnRef<'a>,
    /// Optional UPC.
    pub upc: Option<&'a str>,
    /// Optional DLNA device class marker, for example `DMS-1.50`.
    pub dlna_doc: Option<&'a str>,
    /// Optional presentation URL.
    pub presentation_url: Option<&'a str>,
    /// Icons exposed by the device.
    pub icons: &'a [IconRef<'a>],
    /// Services exposed by the device.
    pub services: &'a [ServiceRef<'a>],
    /// Embedded child devices.
    pub devices: &'a [DeviceRef<'a>],
}

/// A complete UPnP device description document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceDescriptionRef<'a> {
    /// Optional base URL for relative URLs in this document.
    pub url_base: Option<&'a str>,
    /// UPnP spec version.
    pub spec_version: SpecVersion,
    /// Root device.
    pub device: DeviceRef<'a>,
}

/// Writes a complete UPnP device description document.
pub fn write_device_description<W: Write + ?Sized>(
    w: &mut W,
    desc: &DeviceDescriptionRef<'_>,
) -> Result<(), WriteError> {
    w.write_str(r#"<?xml version="1.0"?>"#)?;
    w.write_str(
        r#"<root xmlns="urn:schemas-upnp-org:device-1-0" xmlns:dlna="urn:schemas-dlna-org:device-1-0">"#,
    )?;
    w.write_str("<specVersion>")?;
    write!(
        w,
        "<major>{}</major><minor>{}</minor>",
        desc.spec_version.major, desc.spec_version.minor
    )?;
    w.write_str("</specVersion>")?;
    if let Some(url_base) = desc.url_base {
        write_text_element(w, "URLBase", url_base)?;
    }
    write_device(w, &desc.device)?;
    w.write_str("</root>")?;
    Ok(())
}

/// Builds a complete UPnP device description document in an owned string.
#[cfg(feature = "alloc")]
pub fn device_description_to_string(
    desc: &DeviceDescriptionRef<'_>,
) -> Result<alloc::string::String, WriteError> {
    collect_to_string(|out| write_device_description(out, desc))
}

fn write_device<W: Write + ?Sized>(w: &mut W, device: &DeviceRef<'_>) -> Result<(), WriteError> {
    w.write_str("<device>")?;
    write_display_element(w, "deviceType", &device.device_type)?;
    write_text_element(w, "friendlyName", device.friendly_name)?;
    write_text_element(w, "manufacturer", device.manufacturer)?;
    write_optional(w, "manufacturerURL", device.manufacturer_url)?;
    write_optional(w, "modelDescription", device.model_description)?;
    write_text_element(w, "modelName", device.model_name)?;
    write_optional(w, "modelNumber", device.model_number)?;
    write_optional(w, "modelURL", device.model_url)?;
    write_optional(w, "serialNumber", device.serial_number)?;
    write_display_element(w, "UDN", &device.udn)?;
    write_optional(w, "UPC", device.upc)?;
    write_optional(w, "dlna:X_DLNADOC", device.dlna_doc)?;
    write_icons(w, device.icons)?;
    write_services(w, device.services)?;
    write_devices(w, device.devices)?;
    write_optional(w, "presentationURL", device.presentation_url)?;
    w.write_str("</device>")?;
    Ok(())
}

fn write_optional<W: Write + ?Sized>(
    w: &mut W,
    name: &str,
    value: Option<&str>,
) -> Result<(), WriteError> {
    if let Some(value) = value {
        write_text_element(w, name, value)?;
    }
    Ok(())
}

fn write_icons<W: Write + ?Sized>(w: &mut W, icons: &[IconRef<'_>]) -> Result<(), WriteError> {
    if icons.is_empty() {
        return Ok(());
    }
    w.write_str("<iconList>")?;
    for icon in icons {
        w.write_str("<icon>")?;
        write_text_element(w, "mimetype", icon.mimetype)?;
        write!(
            w,
            "<width>{}</width><height>{}</height><depth>{}</depth>",
            icon.width, icon.height, icon.depth
        )?;
        write_text_element(w, "url", icon.url)?;
        w.write_str("</icon>")?;
    }
    w.write_str("</iconList>")?;
    Ok(())
}

fn write_services<W: Write + ?Sized>(
    w: &mut W,
    services: &[ServiceRef<'_>],
) -> Result<(), WriteError> {
    if services.is_empty() {
        return Ok(());
    }
    w.write_str("<serviceList>")?;
    for service in services {
        w.write_str("<service>")?;
        write_display_element(w, "serviceType", &service.service_type)?;
        write_text_element(w, "serviceId", service.service_id)?;
        write_text_element(w, "SCPDURL", service.scpd_url)?;
        write_text_element(w, "controlURL", service.control_url)?;
        write_text_element(w, "eventSubURL", service.event_sub_url)?;
        w.write_str("</service>")?;
    }
    w.write_str("</serviceList>")?;
    Ok(())
}

fn write_devices<W: Write + ?Sized>(
    w: &mut W,
    devices: &[DeviceRef<'_>],
) -> Result<(), WriteError> {
    if devices.is_empty() {
        return Ok(());
    }
    w.write_str("<deviceList>")?;
    for device in devices {
        write_device(w, device)?;
    }
    w.write_str("</deviceList>")?;
    Ok(())
}

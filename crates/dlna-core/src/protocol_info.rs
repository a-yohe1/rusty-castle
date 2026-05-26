//! DLNA `protocolInfo` and media feature helpers.

use core::fmt::{self, Write};

/// DLNA media profile names useful for initial MediaServer compatibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DlnaProfile {
    /// MPEG-4 AVC MP4 profile.
    AvcMp4BlCif15Aac520,
    /// MPEG PS PAL profile, often suitable for `.mpg` and `.vob`.
    MpegPsPal,
    /// MPEG PS NTSC profile, often suitable for `.mpg` and `.vob`.
    MpegPsNtsc,
}

impl DlnaProfile {
    /// Returns the DLNA.ORG_PN token.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AvcMp4BlCif15Aac520 => "AVC_MP4_BL_CIF15_AAC_520",
            Self::MpegPsPal => "MPEG_PS_PAL",
            Self::MpegPsNtsc => "MPEG_PS_NTSC",
        }
    }
}

/// DLNA operation flags for time/range seeking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DlnaOp {
    /// Byte range requests are supported.
    pub range: bool,
    /// Time seek requests are supported.
    pub time_seek: bool,
}

impl DlnaOp {
    /// Byte range only, encoded as `01`.
    pub const RANGE: Self = Self {
        range: true,
        time_seek: false,
    };

    /// No special operation support, encoded as `00`.
    pub const NONE: Self = Self {
        range: false,
        time_seek: false,
    };

    fn code(self) -> &'static str {
        match (self.time_seek, self.range) {
            (false, false) => "00",
            (false, true) => "01",
            (true, false) => "10",
            (true, true) => "11",
        }
    }
}

/// Raw DLNA.ORG_FLAGS value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DlnaFlags {
    /// 32-bit flag field.
    pub bits: u32,
}

impl DlnaFlags {
    /// Conservative streaming flags accepted by many DLNA renderers.
    pub const STREAMING_TRANSFER_MODE: Self = Self { bits: 0x0170_0000 };
}

/// A borrowed DLNA/UPnP `protocolInfo` value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolInfoRef<'a> {
    /// Transport protocol, commonly `http-get`.
    pub protocol: &'a str,
    /// Network field, commonly `*`.
    pub network: &'a str,
    /// MIME content format.
    pub content_format: &'a str,
    /// Optional DLNA profile.
    pub profile: Option<DlnaProfile>,
    /// Optional DLNA operation value.
    pub op: Option<DlnaOp>,
    /// Optional DLNA flags.
    pub flags: Option<DlnaFlags>,
}

impl<'a> ProtocolInfoRef<'a> {
    /// Creates an HTTP GET protocolInfo value for an MP4 resource.
    pub fn sony_mp4() -> Self {
        Self {
            protocol: "http-get",
            network: "*",
            content_format: "video/mp4",
            profile: Some(DlnaProfile::AvcMp4BlCif15Aac520),
            op: Some(DlnaOp::RANGE),
            flags: Some(DlnaFlags::STREAMING_TRANSFER_MODE),
        }
    }

    /// Creates an HTTP GET protocolInfo value for an MPEG program stream resource.
    pub fn sony_mpeg_ps(content_format: &'a str) -> Self {
        Self {
            protocol: "http-get",
            network: "*",
            content_format,
            profile: Some(DlnaProfile::MpegPsNtsc),
            op: Some(DlnaOp::RANGE),
            flags: Some(DlnaFlags::STREAMING_TRANSFER_MODE),
        }
    }
}

impl fmt::Display for ProtocolInfoRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:",
            self.protocol, self.network, self.content_format
        )?;
        write_additional_info(f, self)
    }
}

/// Writes a `protocolInfo` string.
pub fn write_protocol_info<W: Write + ?Sized>(
    w: &mut W,
    value: &ProtocolInfoRef<'_>,
) -> fmt::Result {
    write!(w, "{value}")
}

/// Writes a `contentFeatures.dlna.org` value from the additionalInfo portion.
pub fn write_content_features<W: Write + ?Sized>(
    w: &mut W,
    value: &ProtocolInfoRef<'_>,
) -> fmt::Result {
    write_additional_info(w, value)
}

/// Builds a `contentFeatures.dlna.org` value in an owned string.
#[cfg(feature = "alloc")]
pub fn content_features_to_string(
    value: &ProtocolInfoRef<'_>,
) -> Result<alloc::string::String, fmt::Error> {
    let mut out = alloc::string::String::new();
    write_content_features(&mut out, value)?;
    Ok(out)
}

fn write_additional_info<W: Write + ?Sized>(w: &mut W, value: &ProtocolInfoRef<'_>) -> fmt::Result {
    let mut wrote = false;
    if let Some(profile) = value.profile {
        write!(w, "DLNA.ORG_PN={}", profile.as_str())?;
        wrote = true;
    }
    if let Some(op) = value.op {
        if wrote {
            w.write_char(';')?;
        }
        write!(w, "DLNA.ORG_OP={}", op.code())?;
        wrote = true;
    }
    if let Some(flags) = value.flags {
        if wrote {
            w.write_char(';')?;
        }
        write!(
            w,
            "DLNA.ORG_FLAGS={:08x}000000000000000000000000",
            flags.bits
        )?;
    }
    Ok(())
}

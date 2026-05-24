//! Encoder for M-SEARCH requests.

use crate::consts::MAN_DISCOVER;
use crate::encode::{BufWriter, write_u32};
use crate::error::EncodeError;
use crate::message::MSearchRef;

/// Encodes an M-SEARCH request into `out` and returns the written slice.
pub fn encode_msearch<'b>(
    msg: &MSearchRef<'_>,
    out: &'b mut [u8],
) -> Result<&'b [u8], EncodeError> {
    let mut w = BufWriter::new(out);
    w.write_str("M-SEARCH * HTTP/1.1\r\n")?;
    w.write_header("HOST", msg.host)?;
    w.write_header("MAN", MAN_DISCOVER)?;
    w.write_str("MX: ")?;
    write_u32(&mut w, u32::from(msg.mx))?;
    w.write_crlf()?;
    encode_target(&mut w, "ST", &msg.st)?;
    if let Some(ua) = msg.user_agent {
        w.write_header("USER-AGENT", ua)?;
    }
    if let Some(v) = msg.cpfn {
        w.write_header("CPFN.UPNP.ORG", v)?;
    }
    if let Some(v) = msg.cpuuid {
        w.write_header("CPUUID.UPNP.ORG", v)?;
    }
    if let Some(port) = msg.tcpport {
        w.write_str("TCPPORT.UPNP.ORG: ")?;
        write_u32(&mut w, u32::from(port))?;
        w.write_crlf()?;
    }
    w.write_crlf()?;
    let n = w.written();
    Ok(&out[..n])
}

pub(crate) fn encode_target(
    w: &mut BufWriter<'_>,
    header: &str,
    target: &crate::header::target::TargetRef<'_>,
) -> Result<(), EncodeError> {
    use crate::header::target::TargetRef;
    w.write_str(header)?;
    w.write_bytes(b": ")?;
    match target {
        TargetRef::All => w.write_str("ssdp:all")?,
        TargetRef::RootDevice => w.write_str("upnp:rootdevice")?,
        TargetRef::Uuid(u) => {
            w.write_str("uuid:")?;
            w.write_str(u)?;
        }
        TargetRef::DeviceType {
            domain,
            kind,
            version,
        } => {
            w.write_str("urn:")?;
            w.write_str(domain)?;
            w.write_str(":device:")?;
            w.write_str(kind)?;
            w.write_bytes(b":")?;
            write_u32(w, *version)?;
        }
        TargetRef::ServiceType {
            domain,
            kind,
            version,
        } => {
            w.write_str("urn:")?;
            w.write_str(domain)?;
            w.write_str(":service:")?;
            w.write_str(kind)?;
            w.write_bytes(b":")?;
            write_u32(w, *version)?;
        }
    }
    w.write_crlf()
}

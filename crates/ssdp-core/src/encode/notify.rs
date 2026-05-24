//! Encoder for NOTIFY messages.

use crate::encode::{BufWriter, msearch::encode_target, write_u32};
use crate::error::EncodeError;
use crate::message::NotifyRef;

/// Encodes a NOTIFY message into `out` and returns the written slice.
pub fn encode_notify<'b>(msg: &NotifyRef<'_>, out: &'b mut [u8]) -> Result<&'b [u8], EncodeError> {
    let mut w = BufWriter::new(out);
    w.write_str("NOTIFY * HTTP/1.1\r\n")?;
    w.write_header("HOST", msg.host)?;
    encode_target(&mut w, "NT", &msg.nt)?;
    w.write_header("NTS", msg.nts.as_str())?;
    encode_usn(&mut w, &msg.usn)?;

    if let Some(loc) = msg.location {
        w.write_header("LOCATION", loc)?;
    }
    if let Some(age) = msg.max_age {
        w.write_str("CACHE-CONTROL: max-age=")?;
        write_u32(&mut w, age.as_secs() as u32)?;
        w.write_crlf()?;
    }
    if let Some(srv) = msg.server {
        w.write_header("SERVER", srv)?;
    }
    if let Some(id) = msg.bootid {
        w.write_str("BOOTID.UPNP.ORG: ")?;
        write_u32(&mut w, id)?;
        w.write_crlf()?;
    }
    if let Some(id) = msg.configid {
        w.write_str("CONFIGID.UPNP.ORG: ")?;
        write_u32(&mut w, id)?;
        w.write_crlf()?;
    }
    if let Some(id) = msg.nextbootid {
        w.write_str("NEXTBOOTID.UPNP.ORG: ")?;
        write_u32(&mut w, id)?;
        w.write_crlf()?;
    }
    if let Some(port) = msg.searchport {
        w.write_str("SEARCHPORT.UPNP.ORG: ")?;
        write_u32(&mut w, u32::from(port))?;
        w.write_crlf()?;
    }
    w.write_crlf()?;
    let n = w.written();
    Ok(&out[..n])
}

pub(crate) fn encode_usn(
    w: &mut BufWriter<'_>,
    usn: &crate::header::usn::UsnRef<'_>,
) -> Result<(), EncodeError> {
    use crate::header::target::TargetRef;
    w.write_str("USN: uuid:")?;
    w.write_str(usn.device_uuid)?;
    if let Some(ref emb) = usn.embedded {
        w.write_str("::")?;
        match emb {
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
    }
    w.write_crlf()
}

//! Encoder for M-SEARCH 200 OK responses.

use crate::encode::{BufWriter, msearch::encode_target, notify::encode_usn, write_u32};
use crate::error::EncodeError;
use crate::message::SearchResponseRef;

/// Encodes an M-SEARCH response into `out` and returns the written slice.
pub fn encode_response<'b>(
    msg: &SearchResponseRef<'_>,
    out: &'b mut [u8],
) -> Result<&'b [u8], EncodeError> {
    let mut w = BufWriter::new(out);
    w.write_str("HTTP/1.1 200 OK\r\n")?;
    w.write_str("CACHE-CONTROL: max-age=")?;
    write_u32(&mut w, msg.max_age.as_secs() as u32)?;
    w.write_crlf()?;
    w.write_header("LOCATION", msg.location)?;
    encode_target(&mut w, "ST", &msg.st)?;
    encode_usn(&mut w, &msg.usn)?;
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
    if let Some(port) = msg.searchport {
        w.write_str("SEARCHPORT.UPNP.ORG: ")?;
        write_u32(&mut w, u32::from(port))?;
        w.write_crlf()?;
    }
    w.write_crlf()?;
    let n = w.written();
    Ok(&out[..n])
}

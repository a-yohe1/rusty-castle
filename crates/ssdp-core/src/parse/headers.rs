//! HTTP header extraction for SSDP messages using `httparse`.

use crate::error::ParseError;
use crate::header::{
    bootid, cache_control, header_eq, man, mx, name, nts::Nts, target::TargetRef, usn::UsnRef,
};
use crate::message::{MSearchRef, NotifyRef, SearchResponseRef};
use crate::uri;

/// Maximum number of HTTP headers parsed per datagram.
const MAX_HEADERS: usize = 32;

/// Parses HTTP headers from `buf` (the datagram after the start line) and assembles an
/// `MSearchRef`.
pub(crate) fn parse_msearch<'a>(buf: &'a [u8]) -> Result<MSearchRef<'a>, ParseError> {
    let mut raw = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let (headers, _) = parse_raw(buf, &mut raw)?;

    let mut host = None::<&str>;
    let mut st = None::<TargetRef<'a>>;
    let mut mx_val = None::<u8>;
    let mut user_agent = None::<&str>;
    let mut cpfn = None::<&str>;
    let mut cpuuid = None::<&str>;
    let mut tcpport = None::<u16>;
    let mut saw_man = false;

    for h in headers {
        let val = core::str::from_utf8(h.value)
            .map_err(|_| ParseError::InvalidHeaderValue("(non-utf8)"))?;
        if header_eq(h.name.as_bytes(), name::HOST) {
            host = Some(val);
        } else if header_eq(h.name.as_bytes(), name::ST) {
            st = Some(TargetRef::parse(val)?);
        } else if header_eq(h.name.as_bytes(), name::MAN) {
            man::validate(val)?;
            saw_man = true;
        } else if header_eq(h.name.as_bytes(), name::MX) {
            mx_val = Some(mx::parse(val)?);
        } else if header_eq(h.name.as_bytes(), name::USER_AGENT) {
            user_agent = Some(val);
        } else if header_eq(h.name.as_bytes(), name::CPFN) {
            cpfn = Some(val);
        } else if header_eq(h.name.as_bytes(), name::CPUUID) {
            cpuuid = Some(val);
        } else if header_eq(h.name.as_bytes(), name::TCPPORT) {
            let p: u16 = val
                .trim()
                .parse()
                .map_err(|_| ParseError::InvalidHeaderValue("TCPPORT.UPNP.ORG"))?;
            tcpport = Some(p);
        }
    }

    if !saw_man {
        return Err(ParseError::MissingHeader("MAN"));
    }

    Ok(MSearchRef {
        host: host.ok_or(ParseError::MissingHeader("HOST"))?,
        st: st.ok_or(ParseError::MissingHeader("ST"))?,
        mx: mx_val.ok_or(ParseError::MissingHeader("MX"))?,
        user_agent,
        cpfn,
        cpuuid,
        tcpport,
    })
}

/// Parses HTTP headers from `buf` and assembles a `NotifyRef`.
pub(crate) fn parse_notify<'a>(buf: &'a [u8]) -> Result<NotifyRef<'a>, ParseError> {
    let mut raw = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let (headers, _) = parse_raw(buf, &mut raw)?;

    let mut host = None::<&str>;
    let mut nt = None::<TargetRef<'a>>;
    let mut nts = None::<Nts>;
    let mut usn = None::<UsnRef<'a>>;
    let mut location = None::<&str>;
    let mut max_age = None;
    let mut server = None::<&str>;
    let mut bootid = None::<u32>;
    let mut configid = None::<u32>;
    let mut nextbootid = None::<u32>;
    let mut searchport = None::<u16>;

    for h in headers {
        let val = core::str::from_utf8(h.value)
            .map_err(|_| ParseError::InvalidHeaderValue("(non-utf8)"))?;
        if header_eq(h.name.as_bytes(), name::HOST) {
            host = Some(val);
        } else if header_eq(h.name.as_bytes(), name::NT) {
            nt = Some(TargetRef::parse(val)?);
        } else if header_eq(h.name.as_bytes(), name::NTS) {
            nts = Some(Nts::parse(val)?);
        } else if header_eq(h.name.as_bytes(), name::USN) {
            usn = Some(UsnRef::parse(val)?);
        } else if header_eq(h.name.as_bytes(), name::LOCATION) {
            let v = val.trim();
            if !uri::is_valid(v) {
                return Err(ParseError::InvalidHeaderValue("LOCATION"));
            }
            location = Some(v);
        } else if header_eq(h.name.as_bytes(), name::CACHE_CONTROL) {
            max_age = Some(cache_control::parse_max_age(val)?);
        } else if header_eq(h.name.as_bytes(), name::SERVER) {
            server = Some(val);
        } else if header_eq(h.name.as_bytes(), name::BOOTID) {
            bootid = Some(bootid::parse_u32("BOOTID.UPNP.ORG", val)?);
        } else if header_eq(h.name.as_bytes(), name::CONFIGID) {
            configid = Some(bootid::parse_u32("CONFIGID.UPNP.ORG", val)?);
        } else if header_eq(h.name.as_bytes(), name::NEXTBOOTID) {
            nextbootid = Some(bootid::parse_u32("NEXTBOOTID.UPNP.ORG", val)?);
        } else if header_eq(h.name.as_bytes(), name::SEARCHPORT) {
            searchport = Some(bootid::parse_searchport(val)?);
        }
    }

    let nts_val = nts.ok_or(ParseError::MissingHeader("NTS"))?;

    // LOCATION and CACHE-CONTROL are required for alive/update but not byebye.
    let location = match nts_val {
        Nts::ByeBye => location,
        _ => Some(location.ok_or(ParseError::MissingHeader("LOCATION"))?),
    };
    let max_age = match nts_val {
        Nts::ByeBye => max_age,
        _ => Some(max_age.ok_or(ParseError::MissingHeader("CACHE-CONTROL"))?),
    };

    Ok(NotifyRef {
        host: host.ok_or(ParseError::MissingHeader("HOST"))?,
        nt: nt.ok_or(ParseError::MissingHeader("NT"))?,
        nts: nts_val,
        usn: usn.ok_or(ParseError::MissingHeader("USN"))?,
        location,
        max_age,
        server,
        bootid,
        configid,
        nextbootid,
        searchport,
    })
}

/// Parses HTTP headers from `buf` and assembles a `SearchResponseRef`.
pub(crate) fn parse_response<'a>(buf: &'a [u8]) -> Result<SearchResponseRef<'a>, ParseError> {
    let mut raw = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let (headers, _) = parse_raw(buf, &mut raw)?;

    let mut st = None::<TargetRef<'a>>;
    let mut usn = None::<UsnRef<'a>>;
    let mut location = None::<&str>;
    let mut max_age = None;
    let mut server = None::<&str>;
    let mut bootid_val = None::<u32>;
    let mut configid_val = None::<u32>;
    let mut searchport_val = None::<u16>;

    for h in headers {
        let val = core::str::from_utf8(h.value)
            .map_err(|_| ParseError::InvalidHeaderValue("(non-utf8)"))?;
        if header_eq(h.name.as_bytes(), name::ST) {
            st = Some(TargetRef::parse(val)?);
        } else if header_eq(h.name.as_bytes(), name::USN) {
            usn = Some(UsnRef::parse(val)?);
        } else if header_eq(h.name.as_bytes(), name::LOCATION) {
            let v = val.trim();
            if !uri::is_valid(v) {
                return Err(ParseError::InvalidHeaderValue("LOCATION"));
            }
            location = Some(v);
        } else if header_eq(h.name.as_bytes(), name::CACHE_CONTROL) {
            max_age = Some(cache_control::parse_max_age(val)?);
        } else if header_eq(h.name.as_bytes(), name::SERVER) {
            server = Some(val);
        } else if header_eq(h.name.as_bytes(), name::BOOTID) {
            bootid_val = Some(bootid::parse_u32("BOOTID.UPNP.ORG", val)?);
        } else if header_eq(h.name.as_bytes(), name::CONFIGID) {
            configid_val = Some(bootid::parse_u32("CONFIGID.UPNP.ORG", val)?);
        } else if header_eq(h.name.as_bytes(), name::SEARCHPORT) {
            searchport_val = Some(bootid::parse_searchport(val)?);
        }
    }

    Ok(SearchResponseRef {
        st: st.ok_or(ParseError::MissingHeader("ST"))?,
        usn: usn.ok_or(ParseError::MissingHeader("USN"))?,
        location: location.ok_or(ParseError::MissingHeader("LOCATION"))?,
        max_age: max_age.ok_or(ParseError::MissingHeader("CACHE-CONTROL"))?,
        server,
        bootid: bootid_val,
        configid: configid_val,
        searchport: searchport_val,
    })
}

/// Invokes `httparse` and returns a slice of valid headers plus the body offset.
fn parse_raw<'buf, 'h>(
    buf: &'buf [u8],
    headers: &'h mut [httparse::Header<'buf>],
) -> Result<(&'h [httparse::Header<'buf>], usize), ParseError> {
    match httparse::parse_headers(buf, headers) {
        Ok(httparse::Status::Complete((offset, hdrs))) => Ok((hdrs, offset)),
        Ok(httparse::Status::Partial) => Err(ParseError::MalformedHeaders),
        Err(_) => Err(ParseError::MalformedHeaders),
    }
}

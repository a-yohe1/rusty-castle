//! SOAP control request parser and response writers.

use crate::error::{ParseError, WriteError};
use crate::ids::ServiceTypeRef;
#[cfg(feature = "alloc")]
use crate::xml::collect_to_string;
use crate::xml::{write_escaped_text, write_text_element};
use core::fmt::Write;

/// A parsed SOAP action request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionRequestRef<'a> {
    /// Service type namespace from the action element.
    pub service_type: ServiceTypeRef<'a>,
    /// Action name without an XML prefix.
    pub action_name: &'a str,
    /// XML body inside the action element.
    pub arguments_xml: &'a str,
}

impl<'a> ActionRequestRef<'a> {
    /// Returns an iterator over direct child text arguments.
    pub fn arguments(&self) -> ArgumentIter<'a> {
        ArgumentIter {
            rest: self.arguments_xml,
        }
    }
}

/// A SOAP action argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgumentRef<'a> {
    /// Argument element name without an XML prefix.
    pub name: &'a str,
    /// Unescaped argument text as it appeared in the source document.
    pub value: &'a str,
}

/// Iterator over direct SOAP action arguments.
#[derive(Clone, Debug)]
pub struct ArgumentIter<'a> {
    rest: &'a str,
}

impl<'a> Iterator for ArgumentIter<'a> {
    type Item = ArgumentRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let start = self.rest.find('<')?;
            let after_lt = &self.rest[start + 1..];
            if after_lt.starts_with('/') || after_lt.starts_with('!') || after_lt.starts_with('?') {
                self.rest = &after_lt[1..];
                continue;
            }
            let tag_end = after_lt.find('>')?;
            let tag = after_lt[..tag_end].trim();
            if tag.ends_with('/') {
                self.rest = &after_lt[tag_end + 1..];
                continue;
            }
            let name = element_name(tag)?;
            let content_start = start + 1 + tag_end + 1;
            let content = &self.rest[content_start..];
            let (close_start, close_len) = find_close_for_name(content, name)?;
            let value = content[..close_start].trim();
            let consumed = content_start + close_start + close_len;
            self.rest = &self.rest[consumed..];
            return Some(ArgumentRef { name, value });
        }
    }
}

/// Parses a SOAP action request from a UTF-8 XML document.
pub fn parse_action_request(input: &str) -> Result<ActionRequestRef<'_>, ParseError> {
    let body = find_element_body(input, "Body").ok_or(ParseError::MissingBody)?;
    let (tag, inner) = first_child_element(body).ok_or(ParseError::MissingAction)?;
    let action_name = element_name(tag).ok_or(ParseError::MalformedXml)?;
    let prefix = element_prefix(tag);
    let namespace = find_namespace(tag, prefix).ok_or(ParseError::MissingNamespace)?;
    let service_type = parse_service_type_urn(namespace)?;
    Ok(ActionRequestRef {
        service_type,
        action_name,
        arguments_xml: inner,
    })
}

/// A SOAP response argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResponseArgumentRef<'a> {
    /// Argument name.
    pub name: &'a str,
    /// Argument value.
    pub value: &'a str,
}

/// Writes a SOAP action response envelope.
pub fn write_action_response<W: Write + ?Sized>(
    w: &mut W,
    service_type: &ServiceTypeRef<'_>,
    action_name: &str,
    arguments: &[ResponseArgumentRef<'_>],
) -> Result<(), WriteError> {
    write_envelope_start(w)?;
    write!(w, r#"<u:{action_name}Response xmlns:u="{service_type}">"#)?;
    for arg in arguments {
        write_text_element(w, arg.name, arg.value)?;
    }
    write!(w, "</u:{action_name}Response>")?;
    write_envelope_end(w)?;
    Ok(())
}

/// Builds a SOAP action response envelope in an owned string.
#[cfg(feature = "alloc")]
pub fn action_response_to_string(
    service_type: &ServiceTypeRef<'_>,
    action_name: &str,
    arguments: &[ResponseArgumentRef<'_>],
) -> Result<alloc::string::String, WriteError> {
    collect_to_string(|out| write_action_response(out, service_type, action_name, arguments))
}

/// Writes a UPnP SOAP fault.
pub fn write_fault<W: Write + ?Sized>(
    w: &mut W,
    error_code: u32,
    error_description: &str,
) -> Result<(), WriteError> {
    write_envelope_start(w)?;
    w.write_str("<s:Fault>")?;
    write_text_element(w, "faultcode", "s:Client")?;
    write_text_element(w, "faultstring", "UPnPError")?;
    w.write_str("<detail><UPnPError xmlns=\"urn:schemas-upnp-org:control-1-0\">")?;
    write!(w, "<errorCode>{error_code}</errorCode><errorDescription>")?;
    write_escaped_text(w, error_description)?;
    w.write_str("</errorDescription></UPnPError></detail></s:Fault>")?;
    write_envelope_end(w)?;
    Ok(())
}

/// Builds a UPnP SOAP fault in an owned string.
#[cfg(feature = "alloc")]
pub fn fault_to_string(
    error_code: u32,
    error_description: &str,
) -> Result<alloc::string::String, WriteError> {
    collect_to_string(|out| write_fault(out, error_code, error_description))
}

fn write_envelope_start<W: Write + ?Sized>(w: &mut W) -> Result<(), WriteError> {
    w.write_str(r#"<?xml version="1.0"?>"#)?;
    w.write_str(r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" "#)?;
    w.write_str(r#"s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/"><s:Body>"#)?;
    Ok(())
}

fn write_envelope_end<W: Write + ?Sized>(w: &mut W) -> Result<(), WriteError> {
    w.write_str("</s:Body></s:Envelope>")?;
    Ok(())
}

fn find_element_body<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = input;
    loop {
        let start = rest.find('<')?;
        let after_lt = &rest[start + 1..];
        let tag_end = after_lt.find('>')?;
        let tag = after_lt[..tag_end].trim();
        if element_name(tag) == Some(name) {
            let content = &after_lt[tag_end + 1..];
            let (close_start, _) = find_close_for_name(content, name)?;
            return Some(&content[..close_start]);
        }
        rest = &after_lt[tag_end + 1..];
    }
}

fn first_child_element(input: &str) -> Option<(&str, &str)> {
    let start = input.find('<')?;
    let after_lt = &input[start + 1..];
    let tag_end = after_lt.find('>')?;
    let tag = after_lt[..tag_end].trim();
    if tag.starts_with('/') || tag.starts_with('!') || tag.starts_with('?') || tag.ends_with('/') {
        return None;
    }
    let name = element_name(tag)?;
    let content = &after_lt[tag_end + 1..];
    let (close_start, _) = find_close_for_name(content, name)?;
    Some((tag, &content[..close_start]))
}

fn element_name(tag: &str) -> Option<&str> {
    let first = tag.split_ascii_whitespace().next()?;
    let name = first.rsplit_once(':').map_or(first, |(_, local)| local);
    if name.is_empty() { None } else { Some(name) }
}

fn element_prefix(tag: &str) -> Option<&str> {
    let first = tag.split_ascii_whitespace().next()?;
    first.split_once(':').map(|(prefix, _)| prefix)
}

fn find_close_for_name(input: &str, name: &str) -> Option<(usize, usize)> {
    let mut rest = input;
    let mut offset = 0;
    loop {
        let start = rest.find("</")?;
        let after = &rest[start + 2..];
        let end = after.find('>')?;
        if element_name(&after[..end]) == Some(name) {
            return Some((offset + start, 2 + end + 1));
        }
        offset += start + 2 + end + 1;
        rest = &after[end + 1..];
    }
}

fn find_namespace<'a>(tag: &'a str, prefix: Option<&str>) -> Option<&'a str> {
    let mut rest = tag;
    loop {
        let idx = rest.find("xmlns")?;
        let candidate = &rest[idx..];
        let attr_len = namespace_attr_len(candidate, prefix);
        if let Some(attr_len) = attr_len {
            if let Some(value) = find_attr_value(&candidate[attr_len..]) {
                return Some(value);
            }
        }
        rest = &candidate[5..];
    }
}

fn namespace_attr_len(candidate: &str, prefix: Option<&str>) -> Option<usize> {
    match prefix {
        Some(prefix) => {
            let after_colon = candidate.strip_prefix("xmlns:")?;
            if !after_colon.starts_with(prefix) {
                return None;
            }
            let attr_len = "xmlns:".len() + prefix.len();
            let after_name = candidate[attr_len..].trim_start();
            after_name.starts_with('=').then_some(attr_len)
        }
        None => {
            let after_name = candidate["xmlns".len()..].trim_start();
            after_name.starts_with('=').then_some("xmlns".len())
        }
    }
}

fn find_attr_value(after_name: &str) -> Option<&str> {
    let after_eq = after_name.trim_start().strip_prefix('=')?.trim_start();
    let quote = after_eq.as_bytes().first().copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let value = &after_eq[1..];
    let end = value.as_bytes().iter().position(|&b| b == quote)?;
    Some(&value[..end])
}

fn parse_service_type_urn(input: &str) -> Result<ServiceTypeRef<'_>, ParseError> {
    let rest = input.strip_prefix("urn:").ok_or(ParseError::InvalidUrn)?;
    let (domain, rest) = rest.split_once(":service:").ok_or(ParseError::InvalidUrn)?;
    let (kind, version) = rest.rsplit_once(':').ok_or(ParseError::InvalidUrn)?;
    let version = version.parse().map_err(|_| ParseError::InvalidUrn)?;
    Ok(ServiceTypeRef {
        domain,
        kind,
        version,
    })
}

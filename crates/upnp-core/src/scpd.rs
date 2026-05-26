//! SCPD service description XML model and writer.

use crate::error::WriteError;
#[cfg(feature = "alloc")]
use crate::xml::collect_to_string;
use crate::xml::write_text_element;
use core::fmt::Write;

/// Direction of an SCPD action argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentDirection {
    /// Input argument.
    In,
    /// Output argument.
    Out,
}

impl ArgumentDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
        }
    }
}

/// An SCPD action argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArgumentRef<'a> {
    /// Argument name.
    pub name: &'a str,
    /// Direction.
    pub direction: ArgumentDirection,
    /// Related state variable name.
    pub related_state_variable: &'a str,
}

/// An SCPD action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionRef<'a> {
    /// Action name.
    pub name: &'a str,
    /// Action arguments.
    pub arguments: &'a [ArgumentRef<'a>],
}

/// An allowed value list for a state variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllowedValueListRef<'a> {
    /// Allowed values.
    pub values: &'a [&'a str],
}

/// An SCPD state variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateVariableRef<'a> {
    /// Whether UPnP events are sent for this variable.
    pub send_events: bool,
    /// Variable name.
    pub name: &'a str,
    /// UPnP data type, for example `string` or `ui4`.
    pub data_type: &'a str,
    /// Optional default value.
    pub default_value: Option<&'a str>,
    /// Optional allowed values.
    pub allowed_values: Option<AllowedValueListRef<'a>>,
}

/// A complete SCPD document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScpdRef<'a> {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Action list.
    pub actions: &'a [ActionRef<'a>],
    /// Service state table.
    pub state_variables: &'a [StateVariableRef<'a>],
}

/// Writes a complete SCPD document.
pub fn write_scpd<W: Write + ?Sized>(w: &mut W, scpd: &ScpdRef<'_>) -> Result<(), WriteError> {
    w.write_str(r#"<?xml version="1.0"?>"#)?;
    w.write_str(r#"<scpd xmlns="urn:schemas-upnp-org:service-1-0">"#)?;
    write!(
        w,
        "<specVersion><major>{}</major><minor>{}</minor></specVersion>",
        scpd.major, scpd.minor
    )?;
    write_actions(w, scpd.actions)?;
    write_state_variables(w, scpd.state_variables)?;
    w.write_str("</scpd>")?;
    Ok(())
}

/// Builds a complete SCPD document in an owned string.
#[cfg(feature = "alloc")]
pub fn scpd_to_string(scpd: &ScpdRef<'_>) -> Result<alloc::string::String, WriteError> {
    collect_to_string(|out| write_scpd(out, scpd))
}

fn write_actions<W: Write + ?Sized>(
    w: &mut W,
    actions: &[ActionRef<'_>],
) -> Result<(), WriteError> {
    if actions.is_empty() {
        return Ok(());
    }
    w.write_str("<actionList>")?;
    for action in actions {
        w.write_str("<action>")?;
        write_text_element(w, "name", action.name)?;
        if !action.arguments.is_empty() {
            w.write_str("<argumentList>")?;
            for arg in action.arguments {
                w.write_str("<argument>")?;
                write_text_element(w, "name", arg.name)?;
                write_text_element(w, "direction", arg.direction.as_str())?;
                write_text_element(w, "relatedStateVariable", arg.related_state_variable)?;
                w.write_str("</argument>")?;
            }
            w.write_str("</argumentList>")?;
        }
        w.write_str("</action>")?;
    }
    w.write_str("</actionList>")?;
    Ok(())
}

fn write_state_variables<W: Write + ?Sized>(
    w: &mut W,
    vars: &[StateVariableRef<'_>],
) -> Result<(), WriteError> {
    if vars.is_empty() {
        return Ok(());
    }
    w.write_str("<serviceStateTable>")?;
    for var in vars {
        write!(
            w,
            r#"<stateVariable sendEvents="{}">"#,
            if var.send_events { "yes" } else { "no" }
        )?;
        write_text_element(w, "name", var.name)?;
        write_text_element(w, "dataType", var.data_type)?;
        if let Some(default_value) = var.default_value {
            write_text_element(w, "defaultValue", default_value)?;
        }
        if let Some(list) = var.allowed_values {
            w.write_str("<allowedValueList>")?;
            for value in list.values {
                write_text_element(w, "allowedValue", value)?;
            }
            w.write_str("</allowedValueList>")?;
        }
        w.write_str("</stateVariable>")?;
    }
    w.write_str("</serviceStateTable>")?;
    Ok(())
}

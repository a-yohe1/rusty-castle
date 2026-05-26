//! Device and service description document builders.

use upnp_av_core::connection_manager::CONNECTION_MANAGER_SERVICE;
use upnp_av_core::content_directory::CONTENT_DIRECTORY_SERVICE;
use upnp_core::device::{
    DeviceDescriptionRef, DeviceRef, ServiceRef, SpecVersion, device_description_to_string,
};
use upnp_core::ids::{DeviceTypeRef, UdnRef};
use upnp_core::scpd::{
    ActionRef, ArgumentDirection, ArgumentRef, ScpdRef, StateVariableRef, scpd_to_string,
};

/// Server identity and URL configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    /// Base URL, for example `http://192.168.1.10:49152/`.
    pub base_url: String,
    /// UUID without the `uuid:` prefix.
    pub uuid: String,
    /// Friendly device name shown to renderers.
    pub friendly_name: String,
    /// Manufacturer name.
    pub manufacturer: String,
    /// Model name.
    pub model_name: String,
}

impl ServerConfig {
    /// Creates a practical default server config.
    pub fn new(
        base_url: impl Into<String>,
        uuid: impl Into<String>,
        friendly_name: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            uuid: uuid.into(),
            friendly_name: friendly_name.into(),
            manufacturer: "upnp-forge".into(),
            model_name: "rusty-castle".into(),
        }
    }
}

/// Builds `/device.xml`.
pub fn device_xml(config: &ServerConfig) -> Result<String, upnp_core::WriteError> {
    let services = [
        ServiceRef {
            service_type: CONTENT_DIRECTORY_SERVICE,
            service_id: "urn:upnp-org:serviceId:ContentDirectory",
            scpd_url: "/ContentDirectory/scpd.xml",
            control_url: "/ContentDirectory/control",
            event_sub_url: "/ContentDirectory/event",
        },
        ServiceRef {
            service_type: CONNECTION_MANAGER_SERVICE,
            service_id: "urn:upnp-org:serviceId:ConnectionManager",
            scpd_url: "/ConnectionManager/scpd.xml",
            control_url: "/ConnectionManager/control",
            event_sub_url: "/ConnectionManager/event",
        },
    ];
    let device = DeviceRef {
        device_type: DeviceTypeRef {
            domain: "schemas-upnp-org",
            kind: "MediaServer",
            version: 1,
        },
        friendly_name: &config.friendly_name,
        manufacturer: &config.manufacturer,
        manufacturer_url: None,
        model_description: Some("Rust UPnP AV MediaServer"),
        model_name: &config.model_name,
        model_number: Some(env!("CARGO_PKG_VERSION")),
        model_url: None,
        serial_number: None,
        udn: UdnRef { uuid: &config.uuid },
        upc: None,
        dlna_doc: Some("DMS-1.50"),
        presentation_url: None,
        icons: &[],
        services: &services,
        devices: &[],
    };
    device_description_to_string(&DeviceDescriptionRef {
        url_base: Some(&config.base_url),
        spec_version: SpecVersion::default(),
        device,
    })
}

/// Builds `/ContentDirectory/scpd.xml`.
pub fn content_directory_scpd_xml() -> Result<String, upnp_core::WriteError> {
    let browse_args = [
        arg("ObjectID", ArgumentDirection::In, "A_ARG_TYPE_ObjectID"),
        arg("BrowseFlag", ArgumentDirection::In, "A_ARG_TYPE_BrowseFlag"),
        arg("Filter", ArgumentDirection::In, "A_ARG_TYPE_Filter"),
        arg("StartingIndex", ArgumentDirection::In, "A_ARG_TYPE_Index"),
        arg("RequestedCount", ArgumentDirection::In, "A_ARG_TYPE_Count"),
        arg(
            "SortCriteria",
            ArgumentDirection::In,
            "A_ARG_TYPE_SortCriteria",
        ),
        arg("Result", ArgumentDirection::Out, "A_ARG_TYPE_Result"),
        arg("NumberReturned", ArgumentDirection::Out, "A_ARG_TYPE_Count"),
        arg("TotalMatches", ArgumentDirection::Out, "A_ARG_TYPE_Count"),
        arg("UpdateID", ArgumentDirection::Out, "A_ARG_TYPE_UpdateID"),
    ];
    let get_caps_args = [arg(
        "SearchCaps",
        ArgumentDirection::Out,
        "A_ARG_TYPE_Filter",
    )];
    let sort_caps_args = [arg(
        "SortCaps",
        ArgumentDirection::Out,
        "A_ARG_TYPE_SortCriteria",
    )];
    let update_id_args = [arg("Id", ArgumentDirection::Out, "A_ARG_TYPE_UpdateID")];
    let actions = [
        ActionRef {
            name: "Browse",
            arguments: &browse_args,
        },
        ActionRef {
            name: "GetSearchCapabilities",
            arguments: &get_caps_args,
        },
        ActionRef {
            name: "GetSortCapabilities",
            arguments: &sort_caps_args,
        },
        ActionRef {
            name: "GetSystemUpdateID",
            arguments: &update_id_args,
        },
    ];
    let vars = [
        var("A_ARG_TYPE_ObjectID", "string"),
        var("A_ARG_TYPE_BrowseFlag", "string"),
        var("A_ARG_TYPE_Filter", "string"),
        var("A_ARG_TYPE_SortCriteria", "string"),
        var("A_ARG_TYPE_Result", "string"),
        var("A_ARG_TYPE_Index", "ui4"),
        var("A_ARG_TYPE_Count", "ui4"),
        var("A_ARG_TYPE_UpdateID", "ui4"),
        StateVariableRef {
            send_events: true,
            name: "SystemUpdateID",
            data_type: "ui4",
            default_value: Some("0"),
            allowed_values: None,
        },
    ];
    scpd_to_string(&ScpdRef {
        major: 1,
        minor: 0,
        actions: &actions,
        state_variables: &vars,
    })
}

/// Builds `/ConnectionManager/scpd.xml`.
pub fn connection_manager_scpd_xml() -> Result<String, upnp_core::WriteError> {
    let protocol_info_args = [
        arg("Source", ArgumentDirection::Out, "SourceProtocolInfo"),
        arg("Sink", ArgumentDirection::Out, "SinkProtocolInfo"),
    ];
    let ids_args = [arg(
        "ConnectionIDs",
        ArgumentDirection::Out,
        "CurrentConnectionIDs",
    )];
    let actions = [
        ActionRef {
            name: "GetProtocolInfo",
            arguments: &protocol_info_args,
        },
        ActionRef {
            name: "GetCurrentConnectionIDs",
            arguments: &ids_args,
        },
    ];
    let vars = [
        var("SourceProtocolInfo", "string"),
        var("SinkProtocolInfo", "string"),
        var("CurrentConnectionIDs", "string"),
    ];
    scpd_to_string(&ScpdRef {
        major: 1,
        minor: 0,
        actions: &actions,
        state_variables: &vars,
    })
}

fn arg<'a>(
    name: &'a str,
    direction: ArgumentDirection,
    related_state_variable: &'a str,
) -> ArgumentRef<'a> {
    ArgumentRef {
        name,
        direction,
        related_state_variable,
    }
}

fn var<'a>(name: &'a str, data_type: &'a str) -> StateVariableRef<'a> {
    StateVariableRef {
        send_events: false,
        name,
        data_type,
        default_value: None,
        allowed_values: None,
    }
}

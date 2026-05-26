use upnp_core::device::{DeviceDescriptionRef, DeviceRef, ServiceRef, SpecVersion};
use upnp_core::ids::{DeviceTypeRef, ServiceTypeRef, UdnRef};
use upnp_core::scpd::{
    ActionRef, ArgumentDirection, ArgumentRef, ScpdRef, StateVariableRef, write_scpd,
};
use upnp_core::soap::{
    ResponseArgumentRef, parse_action_request, write_action_response, write_fault,
};

#[test]
fn writes_device_description() {
    let service_type = ServiceTypeRef {
        domain: "schemas-upnp-org",
        kind: "ContentDirectory",
        version: 1,
    };
    let services = [ServiceRef {
        service_type,
        service_id: "urn:upnp-org:serviceId:ContentDirectory",
        scpd_url: "/ContentDirectory/scpd.xml",
        control_url: "/ContentDirectory/control",
        event_sub_url: "/ContentDirectory/event",
    }];
    let device = DeviceRef {
        device_type: DeviceTypeRef {
            domain: "schemas-upnp-org",
            kind: "MediaServer",
            version: 1,
        },
        friendly_name: "Rusty Castle & Library",
        manufacturer: "upnp-forge",
        manufacturer_url: None,
        model_description: Some("MediaServer"),
        model_name: "rusty-castle",
        model_number: None,
        model_url: None,
        serial_number: None,
        udn: UdnRef {
            uuid: "550e8400-e29b-41d4-a716-446655440000",
        },
        upc: None,
        dlna_doc: Some("DMS-1.50"),
        presentation_url: None,
        icons: &[],
        services: &services,
        devices: &[],
    };
    let mut xml = String::new();
    upnp_core::device::write_device_description(
        &mut xml,
        &DeviceDescriptionRef {
            url_base: Some("http://192.168.1.10:49152/"),
            spec_version: SpecVersion::default(),
            device,
        },
    )
    .unwrap();

    assert!(xml.contains("<friendlyName>Rusty Castle &amp; Library</friendlyName>"));
    assert!(xml.contains("<deviceType>urn:schemas-upnp-org:device:MediaServer:1</deviceType>"));
    assert!(xml.contains("<dlna:X_DLNADOC>DMS-1.50</dlna:X_DLNADOC>"));
    assert!(
        xml.contains("<serviceType>urn:schemas-upnp-org:service:ContentDirectory:1</serviceType>")
    );
}

#[test]
fn writes_scpd() {
    let args = [ArgumentRef {
        name: "ObjectID",
        direction: ArgumentDirection::In,
        related_state_variable: "A_ARG_TYPE_ObjectID",
    }];
    let actions = [ActionRef {
        name: "Browse",
        arguments: &args,
    }];
    let vars = [StateVariableRef {
        send_events: false,
        name: "A_ARG_TYPE_ObjectID",
        data_type: "string",
        default_value: None,
        allowed_values: None,
    }];
    let mut xml = String::new();
    write_scpd(
        &mut xml,
        &ScpdRef {
            major: 1,
            minor: 0,
            actions: &actions,
            state_variables: &vars,
        },
    )
    .unwrap();

    assert!(xml.contains("<name>Browse</name>"));
    assert!(xml.contains(r#"<stateVariable sendEvents="no">"#));
}

#[test]
fn parses_soap_action_and_arguments() {
    let xml = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
      <ObjectID>0</ObjectID>
      <BrowseFlag>BrowseDirectChildren</BrowseFlag>
    </u:Browse>
  </s:Body>
</s:Envelope>"#;

    let req = parse_action_request(xml).unwrap();
    assert_eq!(req.action_name, "Browse");
    assert_eq!(req.service_type.kind, "ContentDirectory");
    let args: Vec<_> = req.arguments().collect();
    assert_eq!(args[0].name, "ObjectID");
    assert_eq!(args[0].value, "0");
    assert_eq!(args[1].name, "BrowseFlag");
}

#[test]
fn writes_soap_response_and_fault() {
    let service_type = ServiceTypeRef {
        domain: "schemas-upnp-org",
        kind: "ContentDirectory",
        version: 1,
    };
    let mut response = String::new();
    write_action_response(
        &mut response,
        &service_type,
        "Browse",
        &[ResponseArgumentRef {
            name: "Result",
            value: "<DIDL-Lite/>",
        }],
    )
    .unwrap();
    assert!(response.contains(
        "<u:BrowseResponse xmlns:u=\"urn:schemas-upnp-org:service:ContentDirectory:1\">"
    ));
    assert!(response.contains("<Result>&lt;DIDL-Lite/&gt;</Result>"));

    let mut fault = String::new();
    write_fault(&mut fault, 401, "Invalid Action").unwrap();
    assert!(fault.contains("<errorCode>401</errorCode>"));
    assert!(fault.contains("<errorDescription>Invalid Action</errorDescription>"));
}

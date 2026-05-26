use rusty_castle::{
    ControlError, MediaItem, ServerConfig, StaticCatalog, content_directory_scpd_xml, device_xml,
    handle_control,
};

fn catalog() -> StaticCatalog {
    StaticCatalog::from_items(vec![MediaItem::mp4(
        "1",
        "Clip & Trailer",
        "http://192.168.1.10:49152/media/1.mp4",
    )])
}

#[test]
fn builds_device_xml() {
    let xml = device_xml(&ServerConfig::new(
        "http://192.168.1.10:49152/",
        "550e8400-e29b-41d4-a716-446655440000",
        "Rusty Castle",
    ))
    .unwrap();

    assert!(xml.contains("<friendlyName>Rusty Castle</friendlyName>"));
    assert!(xml.contains("urn:schemas-upnp-org:device:MediaServer:1"));
    assert!(xml.contains("/ContentDirectory/control"));
    assert!(xml.contains("/ConnectionManager/control"));
}

#[test]
fn builds_content_directory_scpd() {
    let xml = content_directory_scpd_xml().unwrap();

    assert!(xml.contains("<name>Browse</name>"));
    assert!(xml.contains("<name>GetSystemUpdateID</name>"));
    assert!(xml.contains("SystemUpdateID"));
}

#[test]
fn handles_browse_direct_children() {
    let soap = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><ObjectID>0</ObjectID><BrowseFlag>BrowseDirectChildren</BrowseFlag><Filter>*</Filter><StartingIndex>0</StartingIndex><RequestedCount>0</RequestedCount><SortCriteria></SortCriteria></u:Browse></s:Body></s:Envelope>"#;

    let response = handle_control(soap, &catalog()).unwrap();
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("<u:BrowseResponse"));
    assert!(response.body.contains("Clip &amp;amp; Trailer"));
    assert!(response.body.contains("<NumberReturned>1</NumberReturned>"));
    assert!(response.body.contains("http-get:*:video/mp4:"));
}

#[test]
fn handles_connection_manager_protocol_info() {
    let soap = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetProtocolInfo xmlns:u="urn:schemas-upnp-org:service:ConnectionManager:1"></u:GetProtocolInfo></s:Body></s:Envelope>"#;

    let response = handle_control(soap, &catalog()).unwrap();
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("<u:GetProtocolInfoResponse"));
    assert!(response.body.contains("DLNA.ORG_PN=AVC_MP4"));
}

#[test]
fn returns_fault_for_missing_object() {
    let soap = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><ObjectID>missing</ObjectID><BrowseFlag>BrowseMetadata</BrowseFlag></u:Browse></s:Body></s:Envelope>"#;

    let err = handle_control(soap, &catalog()).unwrap_err();
    assert_eq!(err, ControlError::NoSuchObject);
    let fault = err.into_response();
    assert_eq!(fault.status_code, 500);
    assert!(fault.body.contains("<errorCode>701</errorCode>"));
}

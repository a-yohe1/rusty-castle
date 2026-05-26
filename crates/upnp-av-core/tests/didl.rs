use dlna_core::ProtocolInfoRef;
use upnp_av_core::connection_manager::{
    write_get_protocol_info_response, write_protocol_info_list,
};
use upnp_av_core::content_directory::{BrowseResponseRef, write_browse_response};
use upnp_av_core::didl::{ObjectRef, ResourceRef, UpnpClass, write_didl};

#[test]
fn writes_didl_video_item() {
    let protocol_info = ProtocolInfoRef::sony_mp4();
    let resources = [ResourceRef {
        url: "http://192.168.1.10:49152/media/1.mp4",
        protocol_info,
        size: Some(1234),
        duration: Some("0:01:02"),
    }];
    let objects = [ObjectRef {
        id: "1",
        parent_id: "0",
        restricted: true,
        title: "Clip & Trailer",
        class: UpnpClass::VideoItem,
        child_count: None,
        resources: &resources,
    }];

    let mut xml = String::new();
    write_didl(&mut xml, &objects).unwrap();
    assert!(xml.contains("<dc:title>Clip &amp; Trailer</dc:title>"));
    assert!(xml.contains("protocolInfo=\"http-get:*:video/mp4:"));
    assert!(xml.contains("object.item.videoItem"));
}

#[test]
fn writes_browse_response() {
    let mut xml = String::new();
    write_browse_response(
        &mut xml,
        &BrowseResponseRef {
            result: "<DIDL-Lite/>",
            number_returned: 1,
            total_matches: 1,
            update_id: 7,
        },
    )
    .unwrap();

    assert!(xml.contains("<Result>&lt;DIDL-Lite/&gt;</Result>"));
    assert!(xml.contains("<NumberReturned>1</NumberReturned>"));
    assert!(xml.contains("<UpdateID>7</UpdateID>"));
}

#[test]
fn writes_connection_manager_protocol_info() {
    let mut source = String::new();
    write_protocol_info_list(&mut source, &[ProtocolInfoRef::sony_mp4()]).unwrap();
    let mut xml = String::new();
    write_get_protocol_info_response(&mut xml, "", &source).unwrap();

    assert!(xml.contains("<u:GetProtocolInfoResponse"));
    assert!(xml.contains("<Source>http-get:*:video/mp4:DLNA.ORG_PN="));
    assert!(xml.contains("<Sink></Sink>"));
}

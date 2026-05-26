use dlna_core::{ProtocolInfoRef, write_content_features};

#[test]
fn writes_sony_mp4_protocol_info() {
    let value = ProtocolInfoRef::sony_mp4();
    let text = value.to_string();

    assert!(text.starts_with("http-get:*:video/mp4:DLNA.ORG_PN=AVC_MP4_BL_CIF15_AAC_520"));
    assert!(text.contains("DLNA.ORG_OP=01"));
    assert!(text.contains("DLNA.ORG_FLAGS=01700000"));
}

#[test]
fn writes_content_features() {
    let value = ProtocolInfoRef::sony_mp4();
    let mut text = String::new();
    write_content_features(&mut text, &value).unwrap();

    assert!(text.starts_with("DLNA.ORG_PN="));
    assert!(!text.starts_with("http-get"));
}

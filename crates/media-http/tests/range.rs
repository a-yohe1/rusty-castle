use dlna_core::ProtocolInfoRef;
use media_http::{
    ByteRangeSpec, MediaHeadersRef, Method, ResponseStatus, SatisfiableRange, parse_range_header,
    plan_media_response,
};

#[test]
fn parses_byte_ranges() {
    assert_eq!(
        parse_range_header("bytes=10-19"),
        Ok(ByteRangeSpec::From {
            first: 10,
            last: Some(19)
        })
    );
    assert_eq!(
        parse_range_header("bytes=10-"),
        Ok(ByteRangeSpec::From {
            first: 10,
            last: None
        })
    );
    assert_eq!(
        parse_range_header("bytes=-500"),
        Ok(ByteRangeSpec::Suffix { len: 500 })
    );
}

#[test]
fn applies_ranges() {
    assert_eq!(
        parse_range_header("bytes=10-99").unwrap().apply(50),
        Ok(SatisfiableRange { start: 10, end: 49 })
    );
    assert_eq!(
        parse_range_header("bytes=-10").unwrap().apply(50),
        Ok(SatisfiableRange { start: 40, end: 49 })
    );
}

#[test]
fn plans_partial_get_and_head() {
    let headers = MediaHeadersRef {
        len: 100,
        content_type: "video/mp4",
        protocol_info: ProtocolInfoRef::sony_mp4(),
    };
    let get = plan_media_response(Method::Get, headers, Some("bytes=5-9"));
    assert_eq!(get.status, ResponseStatus::PartialContent);
    assert!(get.send_body);
    assert_eq!(get.content_length, 5);

    let head = plan_media_response(Method::Head, headers, Some("bytes=5-9"));
    assert_eq!(head.status, ResponseStatus::PartialContent);
    assert!(!head.send_body);
}

#[test]
fn plans_unsatisfiable_range() {
    let headers = MediaHeadersRef {
        len: 100,
        content_type: "video/mp4",
        protocol_info: ProtocolInfoRef::sony_mp4(),
    };
    let plan = plan_media_response(Method::Get, headers, Some("bytes=200-300"));
    assert_eq!(plan.status, ResponseStatus::RangeNotSatisfiable);
    assert_eq!(plan.content_length, 0);
    assert!(plan.content_range.is_some());
}

#[test]
fn plans_empty_media_without_body_range() {
    let headers = MediaHeadersRef {
        len: 0,
        content_type: "video/mp4",
        protocol_info: ProtocolInfoRef::sony_mp4(),
    };
    let plan = plan_media_response(Method::Get, headers, None);

    assert_eq!(plan.status, ResponseStatus::Ok);
    assert_eq!(plan.content_length, 0);
    assert_eq!(plan.body_range, None);
}

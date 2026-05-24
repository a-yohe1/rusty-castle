//! Encode → parse roundtrip tests.

use core::time::Duration;
use ssdp_core::encode::{encode_msearch, encode_notify, encode_response};
use ssdp_core::header::nts::Nts;
use ssdp_core::header::target::TargetRef;
use ssdp_core::header::usn::UsnRef;
use ssdp_core::message::{MSearchRef, MessageRef, NotifyRef, SearchResponseRef};
use ssdp_core::parse::parse_datagram;

fn roundtrip_msearch(msg: &MSearchRef<'_>) {
    let mut buf = [0u8; 1024];
    let enc = encode_msearch(msg, &mut buf).expect("encode_msearch");
    let parsed = parse_datagram(enc).expect("parse_datagram");
    let MessageRef::Search(parsed) = parsed else {
        panic!("expected Search, got {:?}", parsed)
    };
    assert_eq!(parsed.host, msg.host);
    assert_eq!(parsed.mx, msg.mx);
    assert_eq!(parsed.st, msg.st);
}

fn roundtrip_notify(msg: &NotifyRef<'_>) {
    let mut buf = [0u8; 1024];
    let enc = encode_notify(msg, &mut buf).expect("encode_notify");
    let parsed = parse_datagram(enc).expect("parse_datagram");
    let MessageRef::Notify(parsed) = parsed else {
        panic!("expected Notify")
    };
    assert_eq!(parsed.nts, msg.nts);
    assert_eq!(parsed.nt, msg.nt);
    assert_eq!(parsed.location, msg.location);
    assert_eq!(parsed.max_age, msg.max_age);
}

fn roundtrip_response(msg: &SearchResponseRef<'_>) {
    let mut buf = [0u8; 1024];
    let enc = encode_response(msg, &mut buf).expect("encode_response");
    let parsed = parse_datagram(enc).expect("parse_datagram");
    let MessageRef::SearchResponse(parsed) = parsed else {
        panic!("expected SearchResponse")
    };
    assert_eq!(parsed.st, msg.st);
    assert_eq!(parsed.location, msg.location);
    assert_eq!(parsed.max_age, msg.max_age);
}

#[test]
fn msearch_all() {
    roundtrip_msearch(&MSearchRef {
        host: "239.255.255.250:1900",
        st: TargetRef::All,
        mx: 3,
        user_agent: None,
        cpfn: None,
        cpuuid: None,
        tcpport: None,
    });
}

#[test]
fn msearch_device_type() {
    roundtrip_msearch(&MSearchRef {
        host: "239.255.255.250:1900",
        st: TargetRef::DeviceType {
            domain: "schemas-upnp-org",
            kind: "MediaServer",
            version: 1,
        },
        mx: 5,
        user_agent: Some("Linux/5.4 UPnP/1.1 test/1"),
        cpfn: None,
        cpuuid: None,
        tcpport: None,
    });
}

#[test]
fn notify_alive_root_device() {
    roundtrip_notify(&NotifyRef {
        host: "239.255.255.250:1900",
        nt: TargetRef::RootDevice,
        nts: Nts::Alive,
        usn: UsnRef {
            device_uuid: "550e8400-e29b-41d4-a716-446655440000",
            embedded: Some(TargetRef::RootDevice),
        },
        location: Some("http://192.168.1.100:49152/desc.xml"),
        max_age: Some(Duration::from_secs(1800)),
        server: Some("Linux/5.4 UPnP/1.1 TestDev/1.0"),
        bootid: Some(1),
        configid: Some(0),
        nextbootid: None,
        searchport: None,
    });
}

#[test]
fn notify_byebye() {
    roundtrip_notify(&NotifyRef {
        host: "239.255.255.250:1900",
        nt: TargetRef::RootDevice,
        nts: Nts::ByeBye,
        usn: UsnRef {
            device_uuid: "550e8400-e29b-41d4-a716-446655440000",
            embedded: Some(TargetRef::RootDevice),
        },
        location: None,
        max_age: None,
        server: None,
        bootid: None,
        configid: None,
        nextbootid: None,
        searchport: None,
    });
}

#[test]
fn response_basic() {
    roundtrip_response(&SearchResponseRef {
        st: TargetRef::All,
        usn: UsnRef {
            device_uuid: "550e8400-e29b-41d4-a716-446655440000",
            embedded: None,
        },
        location: "http://192.168.1.100:49152/desc.xml",
        max_age: Duration::from_secs(1800),
        server: Some("Linux/5.4 UPnP/1.1 TestDev/1.0"),
        bootid: Some(42),
        configid: Some(7),
        searchport: None,
    });
}

#[test]
fn response_with_searchport() {
    roundtrip_response(&SearchResponseRef {
        st: TargetRef::ServiceType {
            domain: "schemas-upnp-org",
            kind: "ContentDirectory",
            version: 1,
        },
        usn: UsnRef {
            device_uuid: "aaaabbbb-cccc-dddd-eeee-ffffffffffff",
            embedded: Some(TargetRef::ServiceType {
                domain: "schemas-upnp-org",
                kind: "ContentDirectory",
                version: 1,
            }),
        },
        location: "http://10.0.0.1:1234/desc.xml",
        max_age: Duration::from_secs(900),
        server: None,
        bootid: None,
        configid: None,
        searchport: Some(5000),
    });
}

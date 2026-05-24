//! End-to-end SSDP state-machine tests derived from pupnp SSDP behavior.

use core::time::Duration;
use ssdp_core::Instant;
use ssdp_core::encode::encode_msearch;
use ssdp_core::header::nts::Nts;
use ssdp_core::header::target::TargetRef;
use ssdp_core::message::{MSearchRef, MessageRef};
use ssdp_core::parse_datagram;
use ssdp_core::state::device::Device;
use ssdp_core::state::{Destination, Transmit};
use std::net::SocketAddr;

const UUID: &str = "550e8400-e29b-41d4-a716-446655440000";
const LOCATION: &str = "http://192.168.1.100:49152/desc.xml";
const SERVER: &str = "Linux/5.4 UPnP/1.1 TestDev/1.0";

fn device_with_targets() -> Device {
    let mut device = Device::new(UUID, LOCATION, Some(SERVER), 7, 11, 0x1234_5678);
    assert!(device.add_target(&TargetRef::RootDevice, Duration::from_secs(1800)));
    assert!(device.add_target(
        &TargetRef::DeviceType {
            domain: "schemas-upnp-org",
            kind: "MediaServer",
            version: 2,
        },
        Duration::from_secs(1800),
    ));
    assert!(device.add_target(
        &TargetRef::ServiceType {
            domain: "schemas-upnp-org",
            kind: "ContentDirectory",
            version: 1,
        },
        Duration::from_secs(1800),
    ));
    device
}

fn msearch(target: TargetRef<'_>, mx: u8) -> Vec<u8> {
    let msg = MSearchRef {
        host: "239.255.255.250:1900",
        st: target,
        mx,
        user_agent: Some("pupnp-e2e/1"),
        cpfn: None,
        cpuuid: None,
        tcpport: None,
    };
    let mut buf = [0u8; 512];
    encode_msearch(&msg, &mut buf)
        .expect("encode m-search")
        .to_vec()
}

fn drain_transmits(device: &mut Device) -> Vec<(Destination, Vec<u8>)> {
    let mut drained = Vec::new();
    let mut buf = [0u8; 1024];
    while let Some(Transmit { dest, payload }) = device.poll_transmit(&mut buf) {
        drained.push((dest, payload.to_vec()));
    }
    drained
}

#[test]
fn msearch_rootdevice_only_receives_rootdevice_response() {
    let mut device = device_with_targets();
    let source: SocketAddr = "192.0.2.10:54321".parse().unwrap();

    device.handle_datagram(Instant::ZERO, source, &msearch(TargetRef::RootDevice, 2));

    let responses = drain_transmits(&mut device);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].0, Destination::Unicast(source));

    let MessageRef::SearchResponse(response) = parse_datagram(&responses[0].1).unwrap() else {
        panic!("expected search response");
    };
    assert_eq!(response.st, TargetRef::RootDevice);
    assert_eq!(response.location, LOCATION);
    assert_eq!(response.max_age, Duration::from_secs(1800));
}

#[test]
fn msearch_lower_device_version_replies_with_requested_version() {
    let mut device = device_with_targets();
    let source: SocketAddr = "192.0.2.10:54321".parse().unwrap();

    device.handle_datagram(
        Instant::ZERO,
        source,
        &msearch(
            TargetRef::DeviceType {
                domain: "schemas-upnp-org",
                kind: "MediaServer",
                version: 1,
            },
            2,
        ),
    );

    let responses = drain_transmits(&mut device);
    assert_eq!(responses.len(), 1);
    let MessageRef::SearchResponse(response) = parse_datagram(&responses[0].1).unwrap() else {
        panic!("expected search response");
    };
    assert_eq!(
        response.st,
        TargetRef::DeviceType {
            domain: "schemas-upnp-org",
            kind: "MediaServer",
            version: 1,
        }
    );
}

#[test]
fn msearch_unmatched_service_type_does_not_reply() {
    let mut device = device_with_targets();
    let source: SocketAddr = "192.0.2.10:54321".parse().unwrap();

    device.handle_datagram(
        Instant::ZERO,
        source,
        &msearch(
            TargetRef::ServiceType {
                domain: "schemas-upnp-org",
                kind: "RenderingControl",
                version: 1,
            },
            2,
        ),
    );

    assert!(drain_transmits(&mut device).is_empty());
}

#[test]
fn initial_alive_burst_keeps_each_target_distinct() {
    let mut device = device_with_targets();
    device.start(Instant::ZERO);
    device.handle_timeout(Instant::ZERO);

    let notifications = drain_transmits(&mut device);
    assert_eq!(notifications.len(), 3);

    let mut saw_root = false;
    let mut saw_device = false;
    let mut saw_service = false;
    for (_, payload) in notifications {
        let MessageRef::Notify(notify) = parse_datagram(&payload).unwrap() else {
            panic!("expected notify");
        };
        assert_eq!(notify.nts, Nts::Alive);
        assert_eq!(notify.location, Some(LOCATION));
        match notify.nt {
            TargetRef::RootDevice => saw_root = true,
            TargetRef::DeviceType {
                domain: "schemas-upnp-org",
                kind: "MediaServer",
                version: 2,
            } => saw_device = true,
            TargetRef::ServiceType {
                domain: "schemas-upnp-org",
                kind: "ContentDirectory",
                version: 1,
            } => saw_service = true,
            other => panic!("unexpected notification target: {other:?}"),
        }
    }

    assert!(saw_root);
    assert!(saw_device);
    assert!(saw_service);
}

#[test]
fn shutdown_emits_byebye_for_each_target_without_location_or_max_age() {
    let mut device = device_with_targets();
    device.shutdown(Instant::ZERO);

    let notifications = drain_transmits(&mut device);
    assert_eq!(notifications.len(), 3);
    for (_, payload) in notifications {
        let MessageRef::Notify(notify) = parse_datagram(&payload).unwrap() else {
            panic!("expected notify");
        };
        assert_eq!(notify.nts, Nts::ByeBye);
        assert_eq!(notify.location, None);
        assert_eq!(notify.max_age, None);
    }
}

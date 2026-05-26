use rusty_castle::runtime::{RuntimeConfig, RuntimeState, scan_media_dir};
use rusty_castle::{ServerConfig, device_xml};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[test]
fn scans_supported_media_files() {
    let dir = tempfile_dir();
    fs::write(dir.join("clip.mp4"), b"1234").unwrap();
    fs::write(dir.join("movie.vob"), b"123456").unwrap();
    fs::write(dir.join("note.txt"), b"ignored").unwrap();

    let media = scan_media_dir(&dir, "http://127.0.0.1:49152/").unwrap();

    assert_eq!(media.len(), 2);
    let clip = media
        .iter()
        .find(|entry| entry.path.ends_with("clip.mp4"))
        .unwrap();
    assert_eq!(clip.item.size, Some(4));
    assert!(clip.item.url.contains("/media/"));
}

#[test]
fn scanned_state_builds_device_xml() {
    let dir = tempfile_dir();
    fs::write(dir.join("clip.mp4"), b"1234").unwrap();
    let config = RuntimeConfig::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 49152),
        "http://127.0.0.1:49152/",
        "550e8400-e29b-41d4-a716-446655440000",
        "Rusty Castle",
        &dir,
    );
    let state = RuntimeState::scan(&config).unwrap();

    let xml = device_xml(&ServerConfig::new(
        "http://127.0.0.1:49152/",
        "550e8400-e29b-41d4-a716-446655440000",
        "Rusty Castle",
    ))
    .unwrap();

    assert!(xml.contains("<URLBase>http://127.0.0.1:49152/</URLBase>"));
    assert_eq!(state.catalog().items().len(), 1);
}

fn tempfile_dir() -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "rusty-castle-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&dir).unwrap();
    dir
}

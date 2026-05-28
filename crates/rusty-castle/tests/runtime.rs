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
    assert!(clip.item.id.starts_with("m-"));
    assert!(!clip.item.url.contains("/media/1"));
    assert_eq!(clip.item.duration, None);
}

#[test]
fn scans_supported_media_files_recursively() {
    let dir = tempfile_dir();
    fs::create_dir(dir.join("Season 1")).unwrap();
    fs::write(dir.join("Season 1").join("episode.mp4"), b"1234").unwrap();

    let media = scan_media_dir(&dir, "http://127.0.0.1:49152/").unwrap();

    assert_eq!(media.len(), 1);
    assert!(media[0].path.ends_with("Season 1/episode.mp4"));
    assert!(media[0].item.parent_id.starts_with("c-"));
}

#[test]
fn scanned_state_exposes_directory_containers() {
    let dir = tempfile_dir();
    fs::create_dir(dir.join("Season 1")).unwrap();
    fs::write(dir.join("Season 1").join("episode.mp4"), b"1234").unwrap();
    let config = RuntimeConfig::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 49152),
        "http://127.0.0.1:49152/",
        "550e8400-e29b-41d4-a716-446655440000",
        "Rusty Castle",
        &dir,
    );

    let state = RuntimeState::scan(&config).unwrap();
    let container = state
        .catalog()
        .containers()
        .iter()
        .find(|container| container.title == "Season 1")
        .unwrap();
    let item = state.catalog().items().first().unwrap();

    assert_eq!(container.parent_id, "0");
    assert_eq!(item.parent_id, container.id);
    assert_eq!(state.catalog().child_count("0"), 1);
    assert_eq!(state.catalog().child_count(&container.id), 1);
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

#[test]
fn scan_uses_stable_ids_from_relative_paths() {
    let dir = tempfile_dir();
    fs::write(dir.join("clip.mp4"), b"1234").unwrap();
    let first = scan_media_dir(&dir, "http://127.0.0.1:49152/").unwrap();
    let first_id = first[0].item.id.clone();
    let first_state = RuntimeState::new(
        ServerConfig::new(
            "http://127.0.0.1:49152/",
            "550e8400-e29b-41d4-a716-446655440000",
            "Rusty Castle",
        ),
        first,
    );
    let first_update_id = first_state.catalog().update_id();

    fs::write(dir.join("another.mp4"), b"5678").unwrap();
    let second = scan_media_dir(&dir, "http://127.0.0.1:49152/").unwrap();
    let second_state = RuntimeState::new(
        ServerConfig::new(
            "http://127.0.0.1:49152/",
            "550e8400-e29b-41d4-a716-446655440000",
            "Rusty Castle",
        ),
        second.clone(),
    );
    let clip = second
        .iter()
        .find(|entry| entry.path.ends_with("clip.mp4"))
        .unwrap();

    assert_eq!(clip.item.id, first_id);
    assert_eq!(
        clip.item.url,
        format!("http://127.0.0.1:49152/media/{first_id}")
    );
    assert_ne!(second_state.catalog().update_id(), first_update_id);
}

#[test]
fn scan_reads_mp4_duration_from_movie_header() {
    let dir = tempfile_dir();
    fs::write(
        dir.join("clip.mp4"),
        minimal_mp4_with_duration(90_500, 1000),
    )
    .unwrap();

    let media = scan_media_dir(&dir, "http://127.0.0.1:49152/").unwrap();

    assert_eq!(media[0].item.duration.as_deref(), Some("0:01:30.500"));
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

fn minimal_mp4_with_duration(duration: u32, timescale: u32) -> Vec<u8> {
    let mut mvhd = Vec::new();
    mvhd.extend([0, 0, 0, 0]); // version and flags
    mvhd.extend(0u32.to_be_bytes()); // creation time
    mvhd.extend(0u32.to_be_bytes()); // modification time
    mvhd.extend(timescale.to_be_bytes());
    mvhd.extend(duration.to_be_bytes());

    let mut moov_payload = Vec::new();
    moov_payload.extend(mp4_box(*b"mvhd", &mvhd));
    mp4_box(*b"moov", &moov_payload)
}

fn mp4_box(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let size = 8 + payload.len() as u32;
    let mut out = Vec::new();
    out.extend(size.to_be_bytes());
    out.extend(kind);
    out.extend(payload);
    out
}

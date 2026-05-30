//! Blocking std runtime adapter for the initial MediaServer.

use crate::catalog::{MediaContainer, MediaItem, StaticCatalog};
use crate::control::handle_control;
use crate::description::{
    ServerConfig, connection_manager_scpd_xml, content_directory_scpd_xml, device_xml,
};
use crate::scenario::{RecordedInteraction, ScenarioRecorder};
use dlna_core::{DlnaProfile, ProtocolInfoRef, write_content_features};
use log::{debug, error, info, warn};
use media_http::{ContentRange, MediaHeadersRef, Method, ResponseStatus, plan_media_response};
use ssdp_core::header::target::TargetRef;
use ssdp_core::message::MessageRef;
use ssdp_core::state::device::{Device, DeviceEvent};
use ssdp_core::state::{Destination, Transmit};
use ssdp_core::{Instant, parse_datagram};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream, UdpSocket};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::thread;
use std::time::{Duration, SystemTime};

const SSDP_MULTICAST: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(239, 255, 255, 250), 1900);

/// Blocking runtime configuration.
#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    /// Address the HTTP server binds to.
    pub http_bind: SocketAddr,
    /// Public base URL advertised in SSDP and device.xml.
    pub public_base_url: String,
    /// IPv4 interface used for SSDP multicast traffic.
    pub ssdp_interface: Ipv4Addr,
    /// UUID without the `uuid:` prefix.
    pub uuid: String,
    /// Friendly device name.
    pub friendly_name: String,
    /// Media directory to scan.
    pub media_dir: PathBuf,
    /// Enables the experimental browser-based observatory UI.
    pub web_ui_enabled: bool,
    /// Optional scenario capture JSONL path.
    pub scenario_capture_path: Option<PathBuf>,
}

impl RuntimeConfig {
    /// Builds a localhost-oriented config.
    pub fn new(
        http_bind: SocketAddr,
        public_base_url: impl Into<String>,
        uuid: impl Into<String>,
        friendly_name: impl Into<String>,
        media_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            http_bind,
            public_base_url: public_base_url.into(),
            ssdp_interface: Ipv4Addr::UNSPECIFIED,
            uuid: uuid.into(),
            friendly_name: friendly_name.into(),
            media_dir: media_dir.into(),
            web_ui_enabled: false,
            scenario_capture_path: None,
        }
    }

    /// Uses a specific IPv4 interface for SSDP multicast.
    pub fn with_ssdp_interface(mut self, interface: Ipv4Addr) -> Self {
        self.ssdp_interface = interface;
        self
    }

    /// Captures HTTP interactions to a JSON Lines scenario file.
    pub fn with_scenario_capture_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.scenario_capture_path = Some(path.into());
        self
    }

    /// Enables the experimental browser-based observatory UI.
    pub fn with_web_ui_enabled(mut self) -> Self {
        self.web_ui_enabled = true;
        self
    }
}

/// A scanned file and its catalog metadata.
#[derive(Clone, Debug)]
pub struct ServedMedia {
    /// Catalog item.
    pub item: MediaItem,
    /// Local filesystem path.
    pub path: PathBuf,
}

/// Shared runtime state.
#[derive(Clone, Debug)]
pub struct RuntimeState {
    config: ServerConfig,
    catalog: StaticCatalog,
    media: Vec<ServedMedia>,
    web_ui_enabled: bool,
    scenario_recorder: Option<ScenarioRecorder>,
}

impl RuntimeState {
    /// Creates state from explicit media entries.
    pub fn new(config: ServerConfig, media: Vec<ServedMedia>) -> Self {
        let catalog =
            StaticCatalog::from_items(media.iter().map(|entry| entry.item.clone()).collect());
        Self {
            config,
            catalog,
            media,
            web_ui_enabled: false,
            scenario_recorder: None,
        }
    }

    /// Scans the configured media directory.
    pub fn scan(config: &RuntimeConfig) -> io::Result<Self> {
        let scanned = scan_media_tree(&config.media_dir, &config.public_base_url)?;
        let catalog = StaticCatalog::from_parts(
            scanned.containers,
            scanned
                .media
                .iter()
                .map(|entry| entry.item.clone())
                .collect(),
        );
        Ok(Self {
            config: ServerConfig::new(
                config.public_base_url.clone(),
                config.uuid.clone(),
                config.friendly_name.clone(),
            ),
            catalog,
            media: scanned.media,
            web_ui_enabled: config.web_ui_enabled,
            scenario_recorder: None,
        })
    }

    fn with_scenario_recorder(mut self, recorder: ScenarioRecorder) -> Self {
        self.scenario_recorder = Some(recorder);
        self
    }

    fn media_by_id(&self, id: &str) -> Option<&ServedMedia> {
        self.media.iter().find(|entry| entry.item.id == id)
    }

    fn media_by_id_or_legacy_index(&self, id: &str) -> Option<&ServedMedia> {
        self.media_by_id(id).or_else(|| {
            let legacy_index = id.parse::<usize>().ok()?.checked_sub(1)?;
            self.media.get(legacy_index)
        })
    }

    /// Returns the scanned catalog.
    pub fn catalog(&self) -> &StaticCatalog {
        &self.catalog
    }

    fn device_location(&self) -> String {
        format!("{}device.xml", self.config.base_url)
    }
}

#[derive(Debug)]
struct SharedRuntimeState {
    config: RuntimeConfig,
    current: RwLock<RuntimeState>,
    scan_status: RwLock<ScanStatus>,
}

impl SharedRuntimeState {
    fn new(config: RuntimeConfig, current: RuntimeState, initial_scan_duration: Duration) -> Self {
        let item_count = current.catalog().items().len();
        Self {
            config,
            current: RwLock::new(current),
            scan_status: RwLock::new(ScanStatus {
                last_scan_time: Some(SystemTime::now()),
                last_scan_duration: Some(initial_scan_duration),
                item_count,
                error: None,
            }),
        }
    }

    fn current(&self) -> io::Result<RwLockReadGuard<'_, RuntimeState>> {
        self.current
            .read()
            .map_err(|_| io::Error::other("runtime state lock poisoned"))
    }

    fn scan_status(&self) -> io::Result<ScanStatus> {
        self.scan_status
            .read()
            .map(|status| status.clone())
            .map_err(|_| io::Error::other("scan status lock poisoned"))
    }

    fn replace_library(&self, next: RuntimeState, duration: Duration) -> io::Result<()> {
        let item_count = next.catalog().items().len();
        {
            let mut current = self
                .current
                .write()
                .map_err(|_| io::Error::other("runtime state lock poisoned"))?;
            *current = next;
        }
        self.update_scan_status(ScanStatus {
            last_scan_time: Some(SystemTime::now()),
            last_scan_duration: Some(duration),
            item_count,
            error: None,
        })
    }

    fn record_scan_error(&self, error: String, duration: Duration) -> io::Result<()> {
        let item_count = self.current()?.catalog().items().len();
        self.update_scan_status(ScanStatus {
            last_scan_time: Some(SystemTime::now()),
            last_scan_duration: Some(duration),
            item_count,
            error: Some(error),
        })
    }

    fn update_scan_status(&self, next: ScanStatus) -> io::Result<()> {
        let mut status = self
            .scan_status
            .write()
            .map_err(|_| io::Error::other("scan status lock poisoned"))?;
        *status = next;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct ScanStatus {
    last_scan_time: Option<SystemTime>,
    last_scan_duration: Option<Duration>,
    item_count: usize,
    error: Option<String>,
}

/// Runs the blocking HTTP server. This function does not return under normal operation.
pub fn run_http(config: RuntimeConfig) -> io::Result<()> {
    let scan_start = std::time::Instant::now();
    let mut state = RuntimeState::scan(&config)?;
    let initial_scan_duration = scan_start.elapsed();
    if let Some(path) = &config.scenario_capture_path {
        info!("capturing compatibility scenario to {}", path.display());
        state = state.with_scenario_recorder(ScenarioRecorder::create(path)?);
    }
    let state = Arc::new(SharedRuntimeState::new(
        config.clone(),
        state,
        initial_scan_duration,
    ));
    let listener = TcpListener::bind(config.http_bind)?;
    info!(
        "http listening on {} with {} media items",
        config.http_bind,
        state.current()?.catalog().items().len()
    );
    for stream in listener.incoming() {
        let state = Arc::clone(&state);
        match stream {
            Ok(stream) => {
                debug!("accepted http connection from {:?}", stream.peer_addr());
                thread::spawn(move || {
                    if let Err(err) = handle_stream(stream, &state) {
                        warn!("http connection failed: {err}");
                    }
                });
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

/// Runs SSDP advertisement and M-SEARCH response loop. This function does not return under normal operation.
pub fn run_ssdp(config: RuntimeConfig) -> io::Result<()> {
    let state = RuntimeState::scan(&config)?;
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 1900))?;
    socket.set_read_timeout(Some(Duration::from_millis(200)))?;
    socket.set_multicast_loop_v4(false)?;
    socket.join_multicast_v4(SSDP_MULTICAST.ip(), &config.ssdp_interface)?;
    info!(
        "ssdp listening on 0.0.0.0:1900, multicast interface {}",
        config.ssdp_interface
    );

    let start = std::time::Instant::now();
    let mut device = ssdp_device(&state, seed_now());
    device.start(to_ssdp_instant(start));
    let mut in_buf = [0u8; 2048];
    let mut out_buf = [0u8; 2048];

    loop {
        let now = to_ssdp_instant(start);
        device.handle_timeout(now);
        drain_ssdp(&socket, &mut device, &mut out_buf)?;

        match socket.recv_from(&mut in_buf) {
            Ok((n, source)) => {
                debug!("received ssdp datagram from {source} ({n} bytes)");
                log_ssdp_datagram(source, &in_buf[..n]);
                device.handle_datagram(to_ssdp_instant(start), source, &in_buf[..n]);
                drain_ssdp_events(&mut device);
                drain_ssdp(&socket, &mut device, &mut out_buf)?;
            }
            Err(err)
                if err.kind() == io::ErrorKind::WouldBlock
                    || err.kind() == io::ErrorKind::TimedOut => {}
            Err(err) => return Err(err),
        }
    }
}

/// Runs HTTP and SSDP in the current process.
pub fn run(config: RuntimeConfig) -> io::Result<()> {
    let ssdp_config = config.clone();
    thread::spawn(move || {
        if let Err(err) = run_ssdp(ssdp_config) {
            error!("ssdp failed: {err}");
        }
    });
    run_http(config)
}

/// Scans a media directory for initially supported files.
pub fn scan_media_dir(media_dir: &Path, public_base_url: &str) -> io::Result<Vec<ServedMedia>> {
    Ok(scan_media_tree(media_dir, public_base_url)?.media)
}

#[derive(Debug)]
struct ScannedMediaTree {
    containers: Vec<MediaContainer>,
    media: Vec<ServedMedia>,
}

fn scan_media_tree(media_dir: &Path, public_base_url: &str) -> io::Result<ScannedMediaTree> {
    debug!(
        "scanning media directory {} with base url {}",
        media_dir.display(),
        public_base_url
    );
    let mut tree = ScannedMediaTree {
        containers: Vec::new(),
        media: Vec::new(),
    };
    scan_media_dir_into(media_dir, media_dir, "0", public_base_url, &mut tree)?;
    info!("media scan complete: {} supported files", tree.media.len());
    Ok(tree)
}

fn scan_media_dir_into(
    root: &Path,
    dir: &Path,
    parent_id: &str,
    public_base_url: &str,
    tree: &mut ScannedMediaTree,
) -> io::Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            let relative_path = path.strip_prefix(root).unwrap_or(&path);
            let id = stable_container_id(relative_path);
            let title = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(id.as_str())
                .to_string();
            info!(
                "found media container id={} title={:?} path={}",
                id,
                title,
                path.display()
            );
            tree.containers
                .push(MediaContainer::new(id.clone(), parent_id, title));
            scan_media_dir_into(root, &path, &id, public_base_url, tree)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let Some(kind) = media_kind(&path) else {
            continue;
        };
        let relative_path = path.strip_prefix(root).unwrap_or(&path);
        let id = stable_media_id(relative_path);
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(id.as_str())
            .to_string();
        let metadata = entry.metadata()?;
        let url = format!("{public_base_url}media/{id}");
        let mut item = match kind {
            MediaKind::Mp4 => MediaItem::mp4(id, title, url),
            MediaKind::MpegPs { mime, profile } => MediaItem {
                id,
                parent_id: parent_id.to_string(),
                title,
                url,
                mime_type: mime.into(),
                size: None,
                duration: None,
                protocol_info: ProtocolInfoRef {
                    protocol: "http-get",
                    network: "*",
                    content_format: mime,
                    profile: Some(profile),
                    op: Some(dlna_core::DlnaOp::RANGE),
                    flags: Some(dlna_core::DlnaFlags::STREAMING_TRANSFER_MODE),
                },
            },
        };
        item.parent_id = parent_id.to_string();
        item.size = Some(metadata.len());
        if matches!(kind, MediaKind::Mp4) {
            item.duration = mp4_duration(&path)?;
        }
        info!(
            "found media id={} title={:?} path={} mime={} size={} duration={:?}",
            item.id,
            item.title,
            path.display(),
            item.mime_type,
            metadata.len(),
            item.duration
        );
        tree.media.push(ServedMedia { item, path });
    }
    Ok(())
}

fn stable_media_id(relative_path: &Path) -> String {
    stable_object_id("m", relative_path)
}

fn stable_container_id(relative_path: &Path) -> String {
    stable_object_id("c", relative_path)
}

fn stable_object_id(prefix: &str, relative_path: &Path) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for component in relative_path.components() {
        if let Component::Normal(value) = component {
            for byte in value.to_string_lossy().as_bytes() {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
            hash ^= u64::from(b'/');
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("{prefix}-{hash:016x}")
}

fn mp4_duration(path: &Path) -> io::Result<Option<String>> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    let Some((timescale, duration)) = find_mp4_mvhd(&mut file, 0, len)? else {
        return Ok(None);
    };
    if timescale == 0 {
        return Ok(None);
    }
    Ok(Some(format_dlna_duration(duration, timescale)))
}

fn find_mp4_mvhd(file: &mut File, mut start: u64, end: u64) -> io::Result<Option<(u32, u64)>> {
    while start + 8 <= end {
        file.seek(SeekFrom::Start(start))?;
        let mut header = [0u8; 8];
        if file.read_exact(&mut header).is_err() {
            return Ok(None);
        }
        let size32 = u32::from_be_bytes(header[0..4].try_into().unwrap()) as u64;
        let box_type = &header[4..8];
        let (box_size, header_len) = match size32 {
            0 => (end.saturating_sub(start), 8),
            1 => {
                let mut large_size = [0u8; 8];
                file.read_exact(&mut large_size)?;
                (u64::from_be_bytes(large_size), 16)
            }
            _ => (size32, 8),
        };
        if box_size < header_len || start.saturating_add(box_size) > end {
            return Ok(None);
        }
        let payload_start = start + header_len;
        let payload_end = start + box_size;
        if box_type == b"mvhd" {
            return read_mvhd(file, payload_start, payload_end);
        }
        if box_type == b"moov" {
            if let Some(duration) = find_mp4_mvhd(file, payload_start, payload_end)? {
                return Ok(Some(duration));
            }
        }
        start += box_size;
    }
    Ok(None)
}

fn read_mvhd(
    file: &mut File,
    payload_start: u64,
    payload_end: u64,
) -> io::Result<Option<(u32, u64)>> {
    if payload_end.saturating_sub(payload_start) < 20 {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(payload_start))?;
    let mut version_and_flags = [0u8; 4];
    file.read_exact(&mut version_and_flags)?;
    match version_and_flags[0] {
        0 => {
            let mut fields = [0u8; 16];
            file.read_exact(&mut fields)?;
            let timescale = u32::from_be_bytes(fields[8..12].try_into().unwrap());
            let duration = u32::from_be_bytes(fields[12..16].try_into().unwrap()) as u64;
            Ok(Some((timescale, duration)))
        }
        1 => {
            if payload_end.saturating_sub(payload_start) < 32 {
                return Ok(None);
            }
            let mut fields = [0u8; 28];
            file.read_exact(&mut fields)?;
            let timescale = u32::from_be_bytes(fields[16..20].try_into().unwrap());
            let duration = u64::from_be_bytes(fields[20..28].try_into().unwrap());
            Ok(Some((timescale, duration)))
        }
        _ => Ok(None),
    }
}

fn format_dlna_duration(duration: u64, timescale: u32) -> String {
    let timescale = u64::from(timescale);
    let total_ms = duration.saturating_mul(1000).saturating_add(timescale / 2) / timescale;
    let total_seconds = total_ms / 1000;
    let millis = total_ms % 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if millis == 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{hours}:{minutes:02}:{seconds:02}.{millis:03}")
    }
}

fn handle_stream(mut stream: TcpStream, state: &SharedRuntimeState) -> io::Result<()> {
    let peer = stream.peer_addr().ok();
    let mut reader = BufReader::new(stream.try_clone()?);
    let request = read_request(&mut reader)?;
    let response = route_request(&request, state)?;
    capture_interaction(state, &request, &response);
    info!(
        "http {} {} -> {} peer={:?}",
        request.method, request.path, response.status, peer
    );
    write_response(&mut stream, response)
}

fn capture_interaction(state: &SharedRuntimeState, request: &HttpRequest, response: &HttpResponse) {
    let Some(recorder) = &state
        .current()
        .ok()
        .and_then(|current| current.scenario_recorder.clone())
    else {
        return;
    };
    let (response_body, omitted_response_body_bytes) = replay_friendly_body(response);
    let interaction = RecordedInteraction {
        method: &request.method,
        path: &request.path,
        request_headers: &request.headers,
        request_body: &request.body,
        response_status: response.status,
        response_content_type: &response.content_type,
        response_headers: &response.headers,
        response_body,
        omitted_response_body_bytes,
    };
    if let Err(err) = recorder.record(&interaction) {
        warn!("failed to capture scenario interaction: {err}");
    }
}

fn replay_friendly_body(response: &HttpResponse) -> (Option<&str>, usize) {
    let is_text = response.content_type.starts_with("text/")
        || response.content_type.contains("xml")
        || response.content_type.contains("json");
    if is_text {
        match std::str::from_utf8(&response.body) {
            Ok(body) => return (Some(body), 0),
            Err(_) => return (None, response.body.len()),
        }
    }
    (None, response.body.len())
}

fn route_request(request: &HttpRequest, state: &SharedRuntimeState) -> io::Result<HttpResponse> {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/device.xml") => xml_response(device_xml(&state.current()?.config)),
        ("GET", "/ContentDirectory/scpd.xml") => xml_response(content_directory_scpd_xml()),
        ("GET", "/ConnectionManager/scpd.xml") => xml_response(connection_manager_scpd_xml()),
        ("GET", "/ui") if state.current()?.web_ui_enabled => ui_index_response(state),
        ("GET", "/ui/library") if state.current()?.web_ui_enabled => ui_library_response(state),
        ("POST", "/ui/rescan") if state.current()?.web_ui_enabled => ui_rescan_response(state),
        ("POST", "/ContentDirectory/control") | ("POST", "/ConnectionManager/control") => {
            let catalog = state.current()?.catalog.clone();
            let response = match handle_control(&request.body, &catalog) {
                Ok(response) => response,
                Err(err) => err.into_response(),
            };
            Ok(HttpResponse::new(
                response.status_code,
                "text/xml; charset=\"utf-8\"",
                response.body.into_bytes(),
            ))
        }
        ("GET", path) | ("HEAD", path) if path.starts_with("/media/") => {
            let id = &path["/media/".len()..];
            let method = if request.method == "HEAD" {
                Method::Head
            } else {
                Method::Get
            };
            media_response(state, id, method, request.header("range"))
        }
        _ => Ok(HttpResponse::new(
            404,
            "text/plain; charset=\"utf-8\"",
            b"not found".to_vec(),
        )),
    }
}

fn ui_index_response(state: &SharedRuntimeState) -> io::Result<HttpResponse> {
    let current = state.current()?;
    let scan_status = state.scan_status()?;
    let body = format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Rusty Castle Observatory</title>
<style>
:root {{ color-scheme: light dark; font-family: system-ui, sans-serif; }}
body {{ margin: 0; background: #f7f7f3; color: #202124; }}
main {{ max-width: 760px; margin: 0 auto; padding: 48px 24px; }}
h1 {{ font-size: 2rem; margin: 0 0 12px; }}
p {{ line-height: 1.5; }}
.metric {{ display: inline-block; margin: 16px 16px 0 0; }}
.metric strong {{ display: block; font-size: 1.75rem; }}
form {{ margin-top: 24px; }}
button {{ padding: 8px 12px; font: inherit; }}
.error {{ color: #b3261e; }}
</style>
</head>
<body>
<main>
<h1>Rusty Castle Observatory</h1>
<p>Experimental DLNA MediaServer inspection surface.</p>
<p><a href="/ui/library">Open library</a></p>
<div class="metric"><strong>{}</strong><span>media items</span></div>
<div class="metric"><strong>{}</strong><span>containers</span></div>
<div class="metric"><strong>{}</strong><span>last scan</span></div>
<form method="post" action="/ui/rescan"><button type="submit">Rescan library</button></form>
{}
</main>
</body>
</html>"#,
        current.catalog.items().len(),
        current.catalog.containers().len(),
        scan_status_summary(&scan_status),
        scan_error_html(&scan_status)
    );
    Ok(HttpResponse::new(
        200,
        "text/html; charset=\"utf-8\"",
        body.into_bytes(),
    ))
}

fn ui_library_response(state: &SharedRuntimeState) -> io::Result<HttpResponse> {
    let current = state.current()?;
    let scan_status = state.scan_status()?;
    let mut containers = String::new();
    for container in current.catalog.containers() {
        containers.push_str("<tr>");
        table_cell(&mut containers, &container.id);
        table_cell(&mut containers, &container.parent_id);
        table_cell(&mut containers, &container.title);
        table_cell(&mut containers, "");
        table_cell(&mut containers, "");
        table_cell(&mut containers, "");
        table_cell(&mut containers, "");
        table_cell(&mut containers, "");
        containers.push_str("</tr>");
    }

    let mut items = String::new();
    for item in current.catalog.items() {
        items.push_str("<tr>");
        table_cell(&mut items, &item.id);
        table_cell(&mut items, &item.parent_id);
        table_cell(&mut items, &item.title);
        table_cell(&mut items, &item.mime_type);
        table_cell(&mut items, &optional_size(item.size));
        table_cell(&mut items, item.duration.as_deref().unwrap_or(""));
        linked_table_cell(&mut items, &item.url);
        table_cell(&mut items, &item.protocol_info.to_string());
        items.push_str("</tr>");
    }

    let body = format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Rusty Castle Library</title>
<style>
:root {{ color-scheme: light dark; font-family: system-ui, sans-serif; }}
body {{ margin: 0; background: #f7f7f3; color: #202124; }}
main {{ max-width: 1200px; margin: 0 auto; padding: 32px 24px; }}
a {{ color: #1a5fb4; }}
h1 {{ font-size: 1.75rem; margin: 16px 0 8px; }}
h2 {{ font-size: 1.1rem; margin: 32px 0 12px; }}
p {{ line-height: 1.5; }}
.summary {{ display: flex; flex-wrap: wrap; gap: 16px; margin: 20px 0; }}
.metric strong {{ display: block; font-size: 1.4rem; }}
.table-wrap {{ overflow-x: auto; border: 1px solid #d7d7cf; background: #fff; }}
table {{ border-collapse: collapse; min-width: 100%; font-size: 0.9rem; }}
th, td {{ border-bottom: 1px solid #e4e4dc; padding: 10px 12px; text-align: left; vertical-align: top; }}
th {{ background: #ededdf; font-weight: 650; white-space: nowrap; }}
td {{ overflow-wrap: anywhere; }}
code {{ font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.85em; }}
@media (prefers-color-scheme: dark) {{
  body {{ background: #191a1a; color: #eeeeea; }}
  a {{ color: #8ab4f8; }}
  .table-wrap {{ border-color: #3a3b3b; background: #202124; }}
  th, td {{ border-bottom-color: #343535; }}
  th {{ background: #2a2b2b; }}
}}
</style>
</head>
<body>
<main>
<p><a href="/ui">Observatory</a></p>
<h1>Library</h1>
<p>Read-only view of the current ContentDirectory catalog.</p>
<form method="post" action="/ui/rescan"><button type="submit">Rescan library</button></form>
<div class="summary">
<div class="metric"><strong>{}</strong><span>containers</span></div>
<div class="metric"><strong>{}</strong><span>media items</span></div>
<div class="metric"><strong>{}</strong><span>update id</span></div>
<div class="metric"><strong>{}</strong><span>last scan</span></div>
<div class="metric"><strong>{}</strong><span>scan duration</span></div>
</div>
{}
<h2>Containers</h2>
<div class="table-wrap">
<table>
<thead><tr><th>Object ID</th><th>Parent ID</th><th>Title</th><th>MIME type</th><th>Size</th><th>Duration</th><th>Media URL</th><th>protocolInfo</th></tr></thead>
<tbody>{containers}</tbody>
</table>
</div>
<h2>Media Items</h2>
<div class="table-wrap">
<table>
<thead><tr><th>Object ID</th><th>Parent ID</th><th>Title</th><th>MIME type</th><th>Size</th><th>Duration</th><th>Media URL</th><th>protocolInfo</th></tr></thead>
<tbody>{items}</tbody>
</table>
</div>
</main>
</body>
</html>"#,
        current.catalog.containers().len(),
        current.catalog.items().len(),
        current.catalog.update_id(),
        scan_status_time(&scan_status),
        scan_status_duration(&scan_status),
        scan_error_html(&scan_status)
    );
    Ok(HttpResponse::new(
        200,
        "text/html; charset=\"utf-8\"",
        body.into_bytes(),
    ))
}

fn ui_rescan_response(state: &SharedRuntimeState) -> io::Result<HttpResponse> {
    match rescan_library(state) {
        Ok(()) => {
            let mut response = HttpResponse::new(303, "text/plain; charset=\"utf-8\"", Vec::new());
            response
                .headers
                .push(("Location".into(), "/ui/library".into()));
            Ok(response)
        }
        Err(err) => {
            let body = format!(
                "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Rescan failed</title></head><body><main><h1>Rescan failed</h1><p>{}</p><p><a href=\"/ui/library\">Back to library</a></p></main></body></html>",
                html_escape(&err.to_string())
            );
            Ok(HttpResponse::new(
                500,
                "text/html; charset=\"utf-8\"",
                body.into_bytes(),
            ))
        }
    }
}

fn rescan_library(state: &SharedRuntimeState) -> io::Result<()> {
    let started = std::time::Instant::now();
    match RuntimeState::scan(&state.config) {
        Ok(mut next) => {
            next.scenario_recorder = state.current()?.scenario_recorder.clone();
            let duration = started.elapsed();
            state.replace_library(next, duration)?;
            info!("manual media rescan completed in {:?}", duration);
            Ok(())
        }
        Err(err) => {
            let duration = started.elapsed();
            let message = err.to_string();
            state.record_scan_error(message.clone(), duration)?;
            warn!(
                "manual media rescan failed after {:?}: {}",
                duration, message
            );
            Err(err)
        }
    }
}

fn scan_status_summary(status: &ScanStatus) -> String {
    match (status.last_scan_time, status.error.as_deref()) {
        (Some(_), Some(_)) => "failed".into(),
        (Some(_), None) => format!("{} items", status.item_count),
        (None, _) => "never".into(),
    }
}

fn scan_status_time(status: &ScanStatus) -> String {
    status
        .last_scan_time
        .map(format_scan_time)
        .unwrap_or_else(|| "never".into())
}

fn format_scan_time(time: SystemTime) -> String {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => format!("{}s since epoch", duration.as_secs()),
        Err(err) => format!("-{}s since epoch", err.duration().as_secs()),
    }
}

fn scan_status_duration(status: &ScanStatus) -> String {
    status
        .last_scan_duration
        .map(|duration| format!("{} ms", duration.as_millis()))
        .unwrap_or_default()
}

fn scan_error_html(status: &ScanStatus) -> String {
    let Some(error) = &status.error else {
        return String::new();
    };
    format!(
        "<p class=\"error\">Last scan error: {}</p>",
        html_escape(error)
    )
}

fn html_escape(value: &str) -> String {
    let mut out = String::new();
    html_escape_into(&mut out, value);
    out
}

fn optional_size(size: Option<u64>) -> String {
    size.map(|value| value.to_string()).unwrap_or_default()
}

fn table_cell(out: &mut String, value: &str) {
    out.push_str("<td>");
    html_escape_into(out, value);
    out.push_str("</td>");
}

fn linked_table_cell(out: &mut String, value: &str) {
    out.push_str("<td>");
    if value.is_empty() {
        out.push_str("</td>");
        return;
    }
    out.push_str("<a href=\"");
    html_escape_into(out, value);
    out.push_str("\">");
    html_escape_into(out, value);
    out.push_str("</a></td>");
}

fn html_escape_into(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}

fn media_response(
    state: &SharedRuntimeState,
    id: &str,
    method: Method,
    range: Option<&str>,
) -> io::Result<HttpResponse> {
    let entry = {
        let current = state.current()?;
        current.media_by_id_or_legacy_index(id).cloned()
    };
    let Some(entry) = entry else {
        warn!("media request for unknown id={id}");
        return Ok(HttpResponse::new(
            404,
            "text/plain; charset=\"utf-8\"",
            b"not found".to_vec(),
        ));
    };
    let len = std::fs::metadata(&entry.path)?.len();
    let plan = plan_media_response(
        method,
        MediaHeadersRef {
            len,
            content_type: &entry.item.mime_type,
            protocol_info: entry.item.protocol_info,
        },
        range,
    );

    let mut response = HttpResponse::empty(status_code(plan.status), plan.content_type);
    response
        .headers
        .push(("Accept-Ranges".into(), plan.accept_ranges.into()));
    response
        .headers
        .push(("transferMode.dlna.org".into(), plan.transfer_mode.into()));
    let mut features = String::new();
    write_content_features(&mut features, &plan.protocol_info)
        .map_err(|_| io::Error::other("failed to format DLNA features"))?;
    response
        .headers
        .push(("contentFeatures.dlna.org".into(), features));
    if let Some(content_range) = plan.content_range {
        response
            .headers
            .push(("Content-Range".into(), format_content_range(content_range)));
    }
    response
        .headers
        .push(("Content-Length".into(), plan.content_length.to_string()));

    if plan.send_body {
        if let Some(range) = plan.body_range {
            let mut file = File::open(&entry.path)?;
            file.seek(SeekFrom::Start(range.start))?;
            let mut limited = file.take(range.len());
            limited.read_to_end(&mut response.body)?;
        }
    }
    debug!(
        "media response id={} method={:?} status={} range={:?} content_length={}",
        id, method, response.status, range, plan.content_length
    );
    Ok(response)
}

fn read_request(reader: &mut BufReader<TcpStream>) -> io::Result<HttpRequest> {
    let mut start_line = String::new();
    reader.read_line(&mut start_line)?;
    let mut parts = start_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    let content_length = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    Ok(HttpRequest {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

fn write_response(stream: &mut TcpStream, response: HttpResponse) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {} {}\r\n",
        response.status,
        reason_phrase(response.status)
    )?;
    write!(stream, "Content-Type: {}\r\n", response.content_type)?;
    for (name, value) in response.headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    write!(stream, "Connection: close\r\n\r\n")?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn xml_response(body: Result<String, upnp_core::WriteError>) -> io::Result<HttpResponse> {
    body.map(|body| HttpResponse::new(200, "text/xml; charset=\"utf-8\"", body.into_bytes()))
        .map_err(|_| io::Error::other("failed to write XML"))
}

fn ssdp_device(state: &RuntimeState, seed: u32) -> Device {
    let bootid = seed.max(1);
    let mut device = Device::new(
        &state.config.uuid,
        &state.device_location(),
        Some("rusty-castle/0.1 UPnP/1.1"),
        bootid,
        1,
        seed,
    );
    let max_age = Duration::from_secs(1800);
    let _ = device.add_target(&TargetRef::RootDevice, max_age);
    let _ = device.add_target(&TargetRef::Uuid(&state.config.uuid), max_age);
    let _ = device.add_target(
        &TargetRef::DeviceType {
            domain: "schemas-upnp-org",
            kind: "MediaServer",
            version: 1,
        },
        max_age,
    );
    let _ = device.add_target(
        &TargetRef::ServiceType {
            domain: "schemas-upnp-org",
            kind: "ContentDirectory",
            version: 1,
        },
        max_age,
    );
    let _ = device.add_target(
        &TargetRef::ServiceType {
            domain: "schemas-upnp-org",
            kind: "ConnectionManager",
            version: 1,
        },
        max_age,
    );
    device
}

fn drain_ssdp(socket: &UdpSocket, device: &mut Device, buf: &mut [u8]) -> io::Result<()> {
    let mut sent = 0usize;
    while let Some(tx) = device.poll_transmit(buf) {
        send_transmit(socket, &tx)?;
        sent += 1;
    }
    if sent == 0 {
        debug!("no ssdp datagrams queued for transmit");
    }
    Ok(())
}

fn send_transmit(socket: &UdpSocket, tx: &Transmit<'_>) -> io::Result<usize> {
    debug!(
        "sending ssdp datagram to {:?} ({} bytes)",
        tx.dest,
        tx.payload.len()
    );
    match tx.dest {
        Destination::MulticastV4 => socket.send_to(tx.payload, SSDP_MULTICAST),
        Destination::Unicast(addr) => socket.send_to(tx.payload, addr),
        Destination::MulticastV6LinkLocal | Destination::MulticastV6SiteLocal => Ok(0),
    }
}

fn drain_ssdp_events(device: &mut Device) {
    while let Some(event) = device.poll_event() {
        match event {
            DeviceEvent::SearchReceived { source } => {
                info!("ssdp m-search received from {source}");
            }
            DeviceEvent::EncodeError(err) => {
                warn!("failed to encode ssdp datagram: {err}");
            }
            _ => {}
        }
    }
}

fn log_ssdp_datagram(source: SocketAddr, payload: &[u8]) {
    match parse_datagram(payload) {
        Ok(MessageRef::Search(search)) => {
            debug!(
                "parsed m-search from {} st={:?} mx={} user_agent={:?}",
                source, search.st, search.mx, search.user_agent
            );
        }
        Ok(MessageRef::Notify(notify)) => {
            debug!(
                "parsed ssdp notify from {} nt={:?} nts={:?} location={:?}",
                source, notify.nt, notify.nts, notify.location
            );
        }
        Ok(MessageRef::SearchResponse(response)) => {
            debug!(
                "parsed ssdp response from {} st={:?} usn={:?} location={:?}",
                source, response.st, response.usn, response.location
            );
        }
        Err(err) => {
            debug!("failed to parse ssdp datagram from {source}: {err}");
        }
    }
}

fn to_ssdp_instant(start: std::time::Instant) -> Instant {
    Instant::from_duration(start.elapsed())
}

fn seed_now() -> u32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(1)
}

#[derive(Clone, Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl HttpRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Clone, Debug)]
struct HttpResponse {
    status: u16,
    content_type: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpResponse {
    fn new(status: u16, content_type: &str, body: Vec<u8>) -> Self {
        let mut response = Self::empty(status, content_type);
        response
            .headers
            .push(("Content-Length".into(), body.len().to_string()));
        response.body = body;
        response
    }

    fn empty(status: u16, content_type: &str) -> Self {
        Self {
            status,
            content_type: content_type.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MediaKind {
    Mp4,
    MpegPs {
        mime: &'static str,
        profile: DlnaProfile,
    },
}

fn media_kind(path: &Path) -> Option<MediaKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "mp4" => Some(MediaKind::Mp4),
        "mpg" | "mpeg" => Some(MediaKind::MpegPs {
            mime: "video/mpeg",
            profile: DlnaProfile::MpegPsNtsc,
        }),
        "vob" => Some(MediaKind::MpegPs {
            mime: "video/mpeg",
            profile: DlnaProfile::MpegPsNtsc,
        }),
        _ => None,
    }
}

fn status_code(status: ResponseStatus) -> u16 {
    match status {
        ResponseStatus::Ok => 200,
        ResponseStatus::PartialContent => 206,
        ResponseStatus::RangeNotSatisfiable => 416,
    }
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        303 => "See Other",
        206 => "Partial Content",
        404 => "Not Found",
        416 => "Range Not Satisfiable",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

fn format_content_range(value: ContentRange) -> String {
    match value {
        ContentRange::Bytes {
            start,
            end,
            complete_len,
        } => format!("bytes {start}-{end}/{complete_len}"),
        ContentRange::Unsatisfied { complete_len } => format!("bytes */{complete_len}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::net::IpAddr;

    #[test]
    fn scenario_capture_keeps_text_response_body() {
        let response = HttpResponse::new(
            200,
            "text/xml; charset=\"utf-8\"",
            b"<root>ok</root>".to_vec(),
        );

        let (body, omitted) = replay_friendly_body(&response);

        assert_eq!(body, Some("<root>ok</root>"));
        assert_eq!(omitted, 0);
    }

    #[test]
    fn scenario_capture_omits_binary_response_body() {
        let response = HttpResponse::new(200, "video/mp4", vec![0, 1, 2, 3]);

        let (body, omitted) = replay_friendly_body(&response);

        assert_eq!(body, None);
        assert_eq!(omitted, 4);
    }

    #[test]
    fn runtime_config_accepts_scenario_capture_path() {
        let config = RuntimeConfig::new(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 49152),
            "http://127.0.0.1:49152/",
            "550e8400-e29b-41d4-a716-446655440000",
            "Rusty Castle",
            ".",
        )
        .with_scenario_capture_path("/tmp/rusty-castle-scenario.jsonl");

        assert_eq!(
            config.scenario_capture_path.as_deref(),
            Some(Path::new("/tmp/rusty-castle-scenario.jsonl"))
        );
    }

    #[test]
    fn runtime_config_accepts_web_ui_enabled() {
        let config = RuntimeConfig::new(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 49152),
            "http://127.0.0.1:49152/",
            "550e8400-e29b-41d4-a716-446655440000",
            "Rusty Castle",
            ".",
        )
        .with_web_ui_enabled();

        assert!(config.web_ui_enabled);
    }

    #[test]
    fn ui_route_is_not_available_by_default() {
        let state = RuntimeState::new(
            ServerConfig::new(
                "http://127.0.0.1:49152/",
                "550e8400-e29b-41d4-a716-446655440000",
                "Rusty Castle",
            ),
            Vec::new(),
        );
        let request = HttpRequest {
            method: "GET".into(),
            path: "/ui".into(),
            headers: Vec::new(),
            body: String::new(),
        };
        let shared = shared_state(state);

        let response = route_request(&request, &shared).unwrap();

        assert_eq!(response.status, 404);
    }

    #[test]
    fn ui_library_route_is_not_available_by_default() {
        let state = RuntimeState::new(
            ServerConfig::new(
                "http://127.0.0.1:49152/",
                "550e8400-e29b-41d4-a716-446655440000",
                "Rusty Castle",
            ),
            Vec::new(),
        );
        let request = HttpRequest {
            method: "GET".into(),
            path: "/ui/library".into(),
            headers: Vec::new(),
            body: String::new(),
        };
        let shared = shared_state(state);

        let response = route_request(&request, &shared).unwrap();

        assert_eq!(response.status, 404);
    }

    #[test]
    fn ui_route_returns_static_html_when_enabled() {
        let mut state = RuntimeState::new(
            ServerConfig::new(
                "http://127.0.0.1:49152/",
                "550e8400-e29b-41d4-a716-446655440000",
                "Rusty Castle",
            ),
            vec![ServedMedia {
                item: MediaItem::mp4("clip", "Clip", "http://127.0.0.1:49152/media/clip"),
                path: PathBuf::from("clip.mp4"),
            }],
        );
        state.web_ui_enabled = true;
        let request = HttpRequest {
            method: "GET".into(),
            path: "/ui".into(),
            headers: Vec::new(),
            body: String::new(),
        };
        let shared = shared_state(state);

        let response = route_request(&request, &shared).unwrap();
        let body = std::str::from_utf8(&response.body).unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/html; charset=\"utf-8\"");
        assert!(body.contains("Rusty Castle Observatory"));
        assert!(body.contains("1</strong><span>media items"));
    }

    #[test]
    fn ui_library_route_renders_catalog_metadata_when_enabled() {
        let mut item = MediaItem::mp4(
            "episode-1",
            "Clip & Trailer",
            "http://127.0.0.1:49152/media/episode-1",
        );
        item.parent_id = "season-1".into();
        item.size = Some(1234);
        item.duration = Some("0:01:30.500".into());
        let catalog = StaticCatalog::from_parts(
            vec![MediaContainer::new("season-1", "0", "Season <1>")],
            vec![item.clone()],
        );
        let state = RuntimeState {
            config: ServerConfig::new(
                "http://127.0.0.1:49152/",
                "550e8400-e29b-41d4-a716-446655440000",
                "Rusty Castle",
            ),
            catalog,
            media: vec![ServedMedia {
                item,
                path: PathBuf::from("episode-1.mp4"),
            }],
            web_ui_enabled: true,
            scenario_recorder: None,
        };
        let request = HttpRequest {
            method: "GET".into(),
            path: "/ui/library".into(),
            headers: Vec::new(),
            body: String::new(),
        };
        let shared = shared_state(state);

        let response = route_request(&request, &shared).unwrap();
        let body = std::str::from_utf8(&response.body).unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/html; charset=\"utf-8\"");
        assert!(body.contains("<h1>Library</h1>"));
        assert!(body.contains("season-1"));
        assert!(body.contains("Season &lt;1&gt;"));
        assert!(body.contains("episode-1"));
        assert!(body.contains("Clip &amp; Trailer"));
        assert!(body.contains("video/mp4"));
        assert!(body.contains(">1234</td>"));
        assert!(body.contains("0:01:30.500"));
        assert!(body.contains("http://127.0.0.1:49152/media/episode-1"));
        assert!(body.contains("http-get:*:video/mp4:DLNA.ORG_PN="));
    }

    #[test]
    fn rescan_replaces_catalog_and_media_atomically() {
        let dir = tempfile_dir();
        fs::write(dir.join("first.mp4"), b"1234").unwrap();
        let config = test_runtime_config(&dir).with_web_ui_enabled();
        let state = RuntimeState::scan(&config).unwrap();
        let shared = SharedRuntimeState::new(config, state, Duration::from_millis(0));

        fs::write(dir.join("second.mp4"), b"5678").unwrap();
        let request = HttpRequest {
            method: "POST".into(),
            path: "/ui/rescan".into(),
            headers: Vec::new(),
            body: String::new(),
        };

        let response = route_request(&request, &shared).unwrap();
        let current = shared.current().unwrap();

        assert_eq!(response.status, 303);
        assert_eq!(
            response
                .headers
                .iter()
                .find(|(name, _)| name == "Location")
                .map(|(_, value)| value.as_str()),
            Some("/ui/library")
        );
        assert_eq!(current.catalog().items().len(), 2);
        assert_eq!(current.media.len(), 2);
    }

    #[test]
    fn failed_rescan_keeps_existing_catalog_and_reports_error() {
        let dir = tempfile_dir();
        fs::write(dir.join("first.mp4"), b"1234").unwrap();
        let mut config = test_runtime_config(&dir).with_web_ui_enabled();
        let state = RuntimeState::scan(&config).unwrap();
        config.media_dir = dir.join("missing");
        let shared = SharedRuntimeState::new(config, state, Duration::from_millis(0));
        let request = HttpRequest {
            method: "POST".into(),
            path: "/ui/rescan".into(),
            headers: Vec::new(),
            body: String::new(),
        };

        let response = route_request(&request, &shared).unwrap();
        let current = shared.current().unwrap();
        let status = shared.scan_status().unwrap();

        assert_eq!(response.status, 500);
        assert_eq!(current.catalog().items().len(), 1);
        assert!(status.error.is_some());
    }

    fn shared_state(state: RuntimeState) -> SharedRuntimeState {
        let config = RuntimeConfig::new(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 49152),
            state.config.base_url.clone(),
            state.config.uuid.clone(),
            state.config.friendly_name.clone(),
            ".",
        );
        SharedRuntimeState::new(config, state, Duration::from_millis(0))
    }

    fn test_runtime_config(media_dir: &Path) -> RuntimeConfig {
        RuntimeConfig::new(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 49152),
            "http://127.0.0.1:49152/",
            "550e8400-e29b-41d4-a716-446655440000",
            "Rusty Castle",
            media_dir,
        )
    }

    fn tempfile_dir() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "rusty-castle-runtime-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }
}

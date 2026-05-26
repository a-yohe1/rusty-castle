//! Blocking std runtime adapter for the initial MediaServer.

use crate::catalog::{MediaItem, StaticCatalog};
use crate::control::handle_control;
use crate::description::{
    ServerConfig, connection_manager_scpd_xml, content_directory_scpd_xml, device_xml,
};
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
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
        }
    }

    /// Uses a specific IPv4 interface for SSDP multicast.
    pub fn with_ssdp_interface(mut self, interface: Ipv4Addr) -> Self {
        self.ssdp_interface = interface;
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
        }
    }

    /// Scans the configured media directory.
    pub fn scan(config: &RuntimeConfig) -> io::Result<Self> {
        let media = scan_media_dir(&config.media_dir, &config.public_base_url)?;
        Ok(Self::new(
            ServerConfig::new(
                config.public_base_url.clone(),
                config.uuid.clone(),
                config.friendly_name.clone(),
            ),
            media,
        ))
    }

    fn media_by_id(&self, id: &str) -> Option<&ServedMedia> {
        self.media.iter().find(|entry| entry.item.id == id)
    }

    /// Returns the scanned catalog.
    pub fn catalog(&self) -> &StaticCatalog {
        &self.catalog
    }

    fn device_location(&self) -> String {
        format!("{}device.xml", self.config.base_url)
    }
}

/// Runs the blocking HTTP server. This function does not return under normal operation.
pub fn run_http(config: RuntimeConfig) -> io::Result<()> {
    let state = Arc::new(RuntimeState::scan(&config)?);
    let listener = TcpListener::bind(config.http_bind)?;
    info!(
        "http listening on {} with {} media items",
        config.http_bind,
        state.catalog().items().len()
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
            eprintln!("SSDP failed: {err}");
        }
    });
    run_http(config)
}

/// Scans a media directory for initially supported files.
pub fn scan_media_dir(media_dir: &Path, public_base_url: &str) -> io::Result<Vec<ServedMedia>> {
    debug!(
        "scanning media directory {} with base url {}",
        media_dir.display(),
        public_base_url
    );
    let mut out = Vec::new();
    let mut next_id = 1u32;
    let mut entries = std::fs::read_dir(media_dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(kind) = media_kind(&path) else {
            continue;
        };
        let id = next_id.to_string();
        next_id += 1;
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
        item.size = Some(metadata.len());
        info!(
            "found media id={} title={:?} path={} mime={} size={}",
            item.id,
            item.title,
            path.display(),
            item.mime_type,
            metadata.len()
        );
        out.push(ServedMedia { item, path });
    }
    info!("media scan complete: {} supported files", out.len());
    Ok(out)
}

fn handle_stream(mut stream: TcpStream, state: &RuntimeState) -> io::Result<()> {
    let peer = stream.peer_addr().ok();
    let mut reader = BufReader::new(stream.try_clone()?);
    let request = read_request(&mut reader)?;
    let response = route_request(&request, state)?;
    info!(
        "http {} {} -> {} peer={:?}",
        request.method, request.path, response.status, peer
    );
    write_response(&mut stream, response)
}

fn route_request(request: &HttpRequest, state: &RuntimeState) -> io::Result<HttpResponse> {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/device.xml") => xml_response(device_xml(&state.config)),
        ("GET", "/ContentDirectory/scpd.xml") => xml_response(content_directory_scpd_xml()),
        ("GET", "/ConnectionManager/scpd.xml") => xml_response(connection_manager_scpd_xml()),
        ("POST", "/ContentDirectory/control") | ("POST", "/ConnectionManager/control") => {
            let response = match handle_control(&request.body, &state.catalog) {
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

fn media_response(
    state: &RuntimeState,
    id: &str,
    method: Method,
    range: Option<&str>,
) -> io::Result<HttpResponse> {
    let Some(entry) = state.media_by_id(id) else {
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

use rusty_castle::runtime::{RuntimeConfig, run};
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::OnceLock;

fn main() -> std::io::Result<()> {
    init_logging();

    let media_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 49152);
    let host = env::var("RUSTY_CASTLE_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let base_url = format!("http://{host}:49152/");
    let uuid = env::var("RUSTY_CASTLE_UUID")
        .unwrap_or_else(|_| "550e8400-e29b-41d4-a716-446655440000".into());
    let friendly_name = env::var("RUSTY_CASTLE_NAME").unwrap_or_else(|_| "Rusty Castle".into());

    let mut config = RuntimeConfig::new(bind, base_url, uuid, friendly_name, media_dir);
    if let Ok(interface) = host.parse::<Ipv4Addr>() {
        config = config.with_ssdp_interface(interface);
    }

    eprintln!(
        "serving {} at {}",
        config.media_dir.display(),
        config.public_base_url
    );
    if !config.ssdp_interface.is_unspecified() {
        eprintln!("using {} for SSDP multicast", config.ssdp_interface);
    }
    run(config)
}

fn init_logging() {
    static LOGGER: SimpleLogger = SimpleLogger;
    static LEVEL: OnceLock<log::LevelFilter> = OnceLock::new();

    let level = LEVEL.get_or_init(|| {
        env::var("RUST_LOG")
            .or_else(|_| env::var("RUSTY_CASTLE_LOG"))
            .ok()
            .and_then(|value| parse_level(&value))
            .unwrap_or(log::LevelFilter::Info)
    });
    log::set_max_level(*level);
    let _ = log::set_logger(&LOGGER);
}

fn parse_level(value: &str) -> Option<log::LevelFilter> {
    let value = value
        .split(',')
        .next_back()
        .and_then(|part| part.split('=').next_back())
        .unwrap_or(value)
        .trim()
        .to_ascii_lowercase();
    match value.as_str() {
        "off" => Some(log::LevelFilter::Off),
        "error" => Some(log::LevelFilter::Error),
        "warn" | "warning" => Some(log::LevelFilter::Warn),
        "info" => Some(log::LevelFilter::Info),
        "debug" => Some(log::LevelFilter::Debug),
        "trace" => Some(log::LevelFilter::Trace),
        _ => None,
    }
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{} {} {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

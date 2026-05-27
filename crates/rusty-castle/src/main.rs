use log::info;
use rusty_castle::runtime::{RuntimeConfig, run};
use std::env;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VERSION: &str = env!("RUSTY_CASTLE_VERSION");

fn main() -> std::io::Result<()> {
    init_logging();

    let media_dir = match env::args_os().nth(1) {
        Some(arg) if arg == OsStr::new("--version") || arg == OsStr::new("-V") => {
            println!("rusty-castle {VERSION}");
            return Ok(());
        }
        Some(arg) => PathBuf::from(arg),
        None => env::current_dir()?,
    };
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

    info!(
        "serving {} at {}",
        config.media_dir.display(),
        config.public_base_url
    );
    if !config.ssdp_interface.is_unspecified() {
        info!("using {} for SSDP multicast", config.ssdp_interface);
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
            eprintln!(
                "{} {} {} {}",
                format_timestamp(SystemTime::now()),
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

fn format_timestamp(now: SystemTime) -> String {
    let millis_since_epoch = match now.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration_to_millis(duration),
        Err(err) => -duration_to_millis(err.duration()),
    };
    let seconds_since_epoch = millis_since_epoch.div_euclid(1000);
    let milliseconds = millis_since_epoch.rem_euclid(1000);
    let days_since_epoch = seconds_since_epoch.div_euclid(86_400);
    let seconds_of_day = seconds_since_epoch.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days_since_epoch as i64);
    let hour = seconds_of_day / 3_600;
    let minute = seconds_of_day % 3_600 / 60;
    let second = seconds_of_day % 60;

    let mut timestamp = String::new();
    let _ = write!(
        timestamp,
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milliseconds:03}Z"
    );
    timestamp
}

fn duration_to_millis(duration: Duration) -> i128 {
    i128::from(duration.as_secs())
        .saturating_mul(1000)
        .saturating_add(i128::from(duration.subsec_millis()))
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let days = days_since_epoch + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }

    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_unix_epoch_timestamp() {
        assert_eq!(format_timestamp(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn formats_timestamp_with_milliseconds() {
        assert_eq!(
            format_timestamp(UNIX_EPOCH + Duration::from_millis(1_704_067_202_345)),
            "2024-01-01T00:00:02.345Z"
        );
    }
}

//! SSDP protocol constants.

use core::net::{Ipv4Addr, Ipv6Addr};

/// SSDP/UPnP multicast IPv4 address.
pub const SSDP_ADDR_V4: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);

/// SSDP/UPnP multicast IPv6 link-local address.
pub const SSDP_ADDR_V6_LL: Ipv6Addr = Ipv6Addr::new(0xFF02, 0, 0, 0, 0, 0, 0, 0x000C);

/// SSDP/UPnP multicast IPv6 site-local address.
pub const SSDP_ADDR_V6_SL: Ipv6Addr = Ipv6Addr::new(0xFF05, 0, 0, 0, 0, 0, 0, 0x000C);

/// Standard SSDP port.
pub const SSDP_PORT: u16 = 1900;

/// Required value for the MAN header in M-SEARCH requests.
pub const MAN_DISCOVER: &str = "\"ssdp:discover\"";

/// NTS value for alive advertisements.
pub const NTS_ALIVE: &str = "ssdp:alive";

/// NTS value for byebye advertisements.
pub const NTS_BYEBYE: &str = "ssdp:byebye";

/// NTS value for update advertisements (UPnP 1.1).
pub const NTS_UPDATE: &str = "ssdp:update";

/// `ssdp:all` search target — discovers all UPnP devices and services.
pub const ST_ALL: &str = "ssdp:all";

/// `upnp:rootdevice` search/notification target.
pub const NT_ROOT_DEVICE: &str = "upnp:rootdevice";

/// Minimum initial BOOTID value when not persisted across restarts.
pub const BOOTID_MIN: u32 = 1;

/// Maximum MX value accepted; requests higher than this are clamped.
pub const MX_MAX: u8 = 5;

/// Minimum MX value; requests below this are rejected as invalid.
pub const MX_MIN: u8 = 1;

/// Number of initial alive announcements per advertisement at startup (UPnP DA 1.1 §1.2.2).
pub const INITIAL_ALIVE_COUNT: usize = 3;

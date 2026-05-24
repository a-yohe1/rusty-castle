# ssdp-core

`ssdp-core` is a Sans-IO, `no_std` Rust implementation of the SSDP parts of
UPnP Device Architecture 1.1.

It provides typed parsing, encoding, and small state machines for SSDP devices
and control points. Socket setup, multicast membership, packet I/O, and runtime
integration are intentionally left to the caller.

## Features

- `no_std` by default, with no `unsafe` code
- Zero-copy parsing into borrowed message types
- Buffer-based encoding into caller-provided byte slices
- Typed SSDP headers such as `ST`, `NT`, `USN`, `NTS`, `MX`, and
  `CACHE-CONTROL`
- Sans-IO `Device` and `ControlPoint` state machines
- Optional `std` feature for `std::error::Error` implementations
- Optional `defmt` feature for embedded logging integrations

## Message Support

The crate currently handles the core SSDP datagrams:

- `M-SEARCH * HTTP/1.1`
- `NOTIFY * HTTP/1.1`
- `HTTP/1.1 200 OK` search responses

UPnP DA 1.1 headers such as `BOOTID.UPNP.ORG`, `CONFIGID.UPNP.ORG`,
`SEARCHPORT.UPNP.ORG`, `CPFN.UPNP.ORG`, `CPUUID.UPNP.ORG`, and
`TCPPORT.UPNP.ORG` are represented where relevant.

## Quick Example

Parse a received UDP datagram:

```rust
use ssdp_core::{parse_datagram, MessageRef};

let datagram = b"M-SEARCH * HTTP/1.1\r\n\
HOST: 239.255.255.250:1900\r\n\
MAN: \"ssdp:discover\"\r\n\
MX: 3\r\n\
ST: ssdp:all\r\n\
\r\n";

match parse_datagram(datagram)? {
    MessageRef::Search(search) => {
        assert_eq!(search.mx, 3);
    }
    MessageRef::Notify(notify) => {
        // Handle an advertisement.
    }
    MessageRef::SearchResponse(response) => {
        // Handle a response to M-SEARCH.
    }
}
# Ok::<(), ssdp_core::ParseError>(())
```

Encode an `M-SEARCH` packet:

```rust
use ssdp_core::encode::encode_msearch;
use ssdp_core::header::target::TargetRef;
use ssdp_core::message::MSearchRef;

let message = MSearchRef {
    host: "239.255.255.250:1900",
    st: TargetRef::All,
    mx: 3,
    user_agent: None,
    cpfn: None,
    cpuuid: None,
    tcpport: None,
};

let mut buffer = [0u8; 512];
let packet = encode_msearch(&message, &mut buffer)?;

// Send `packet` to the SSDP multicast address with your UDP stack.
# Ok::<(), ssdp_core::EncodeError>(())
```

## Sans-IO State Machines

The state machines follow a poll-style pattern:

1. Feed input with `handle_datagram` or `handle_timeout`.
2. Drain outgoing packets with `poll_transmit`.
3. Drain application events with `poll_event`.
4. Ask `poll_timeout` when the next timer wakeup is needed.

`Device` manages alive advertisements, byebye notifications, and responses to
`M-SEARCH`. `ControlPoint` sends searches, collects responses and notifications,
and expires cached services based on `max-age`.

All real I/O remains outside the crate, so the same protocol core can be used
from embedded network stacks, async runtimes, or tests.

## Feature Flags

| Feature | Description |
| ------- | ----------- |
| `alloc` | Reserved for future owned types |
| `std` | Enables `alloc` and implements `std::error::Error` |
| `defmt` | Enables optional `defmt` support |

## Development

Run the test suite with:

```sh
cargo test
```

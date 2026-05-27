# rusty-castle

Rust-based UPnP AV / DLNA MediaServer workbench, built around reusable
sans-IO protocol crates.

The current implementation can:

- advertise a UPnP MediaServer over SSDP
- expose DLNA MediaServer metadata accepted by Sony TV media players
- serve `/device.xml`
- serve ContentDirectory and ConnectionManager SCPD XML
- handle SOAP control requests for Browse, GetSystemUpdateID, GetProtocolInfo,
  and GetCurrentConnectionIDs
- generate DIDL-Lite results for a static media catalog
- serve media files over HTTP with GET, HEAD, byte ranges, and DLNA headers
- scan `mp4`, `mpg`, `mpeg`, and `vob` files from a media directory

The protocol crates stay `no_std` where practical. Runtime work such as UDP,
TCP, filesystem scanning, and blocking HTTP serving lives in the application
crate.

## Quick Start

Run the MediaServer against a media directory:

```sh
RUSTY_CASTLE_HOST=192.168.1.10 cargo run -p rusty-castle -- /path/to/media
```

`RUSTY_CASTLE_HOST` should be the LAN address that TVs and other DLNA clients
can reach. The server listens on:

- HTTP: `0.0.0.0:49152`
- SSDP: `0.0.0.0:1900`

Optional environment variables:

```sh
RUSTY_CASTLE_UUID=550e8400-e29b-41d4-a716-446655440000
RUSTY_CASTLE_NAME="Rusty Castle"
RUST_LOG=debug
```

`RUST_LOG=debug` enables verbose startup, SSDP, HTTP, SOAP, and media-serving
logs. `RUSTY_CASTLE_LOG=debug` is also accepted when you only want to configure
this binary.

If a TV appears to cache an older device description, change
`RUSTY_CASTLE_UUID` for the next run. The server also changes its SSDP
`BOOTID.UPNP.ORG` on each process start so clients are nudged to refetch
`/device.xml`.

You can inspect the advertised device document directly:

```sh
curl http://192.168.1.10:49152/device.xml
```

List the media exposed through ContentDirectory:

```sh
curl -s \
  -H 'Content-Type: text/xml; charset="utf-8"' \
  -H 'SOAPAction: "urn:schemas-upnp-org:service:ContentDirectory:1#Browse"' \
  --data '<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><ObjectID>0</ObjectID><BrowseFlag>BrowseDirectChildren</BrowseFlag><Filter>*</Filter><StartingIndex>0</StartingIndex><RequestedCount>0</RequestedCount><SortCriteria></SortCriteria></u:Browse></s:Body></s:Envelope>' \
  http://192.168.1.10:49152/ContentDirectory/control
```

Check a media resource and byte-range response:

```sh
curl -I http://192.168.1.10:49152/media/1
curl -r 0-1023 http://192.168.1.10:49152/media/1 -o /tmp/rusty-castle-sample.bin
```

SSDP uses UDP port 1900, so startup can fail if another UPnP service is already
bound to that port.

During successful Sony TV discovery, debug logs should show the TV fetching the
device and service descriptions, then browsing the ContentDirectory:

```text
http GET /device.xml -> 200
http GET /ContentDirectory/scpd.xml -> 200
http GET /ConnectionManager/scpd.xml -> 200
soap action service=ContentDirectory action=Browse
```

## Workspace

```text
crates/
  ssdp-core/       SSDP parser, encoder, and sans-IO state machines
  upnp-core/       Device XML, SCPD XML, SOAP parser/writers, UPnP identifiers
  upnp-av-core/    ContentDirectory, ConnectionManager, DIDL-Lite helpers
  dlna-core/       DLNA protocolInfo, flags, and content feature helpers
  media-http/      HTTP Range parsing and media response planning
  rusty-castle/    Application core, filesystem catalog, blocking std runtime
```

## Architecture

```text
+-----------------------------+
|        rusty-castle         |
| app core + blocking runtime |
+-----------------------------+
              |
+-----------------------------+
|         media-http          |
| Range and media responses   |
+-----------------------------+
              |
+-----------------------------+
|        upnp-av-core         |
| ContentDirectory / DIDL     |
+-----------------------------+
              |
+-----------------------------+
|         upnp-core           |
| Device XML / SCPD / SOAP    |
+-----------------------------+
              |
+-----------------------------+
|         ssdp-core           |
| SSDP sans-IO                |
+-----------------------------+
```

`dlna-core` is used by both `upnp-av-core` and `media-http` for DLNA
`protocolInfo` and HTTP content feature values.

## Supported Media

Initial compatibility targets:

- `.mp4`
- `.mpg`
- `.mpeg`
- `.vob`

The runtime currently exposes files from one flat directory. Recursive scanning,
metadata extraction, subtitles, transcoding, DVD menu support, and AVTransport
are still deferred.

## Development

Install the task runner used by CI:

```sh
cargo install cargo-make
```

Run the same checks as GitHub Actions:

```sh
cargo make ci
```

Run all tests:

```sh
cargo test
```

Run clippy:

```sh
cargo clippy --all-targets --all-features
```

Check the no-default-features media HTTP core:

```sh
cargo test -p media-http --no-default-features
```

Useful crate-level checks:

```sh
cargo test -p ssdp-core
cargo test -p upnp-core --no-default-features
cargo test -p dlna-core --no-default-features
cargo test -p upnp-av-core --no-default-features
```

## Status

Implemented milestones:

- M1: SSDP integration
- M2: `device.xml`
- M3: SOAP
- M4: Browse
- M5: DIDL-Lite
- M6: HTTP Range
- M7: Sony TV discovery, browsing, and MP4 playback validation
- M8: filesystem catalog, initial flat-directory form

Next likely work:

- improve network interface selection for SSDP multicast
- add recursive catalog scanning
- add richer media metadata and DLNA profile selection
- replace or supplement the blocking runtime with an async adapter

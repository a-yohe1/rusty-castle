# rusty-castle

Rust-based UPnP AV / DLNA MediaServer workbench, built around reusable
sans-IO protocol crates.

`rusty-castle` is in early development. It is intended for experimentation,
local-network testing, and feedback from people who want a small Rust DLNA
server stack to inspect and build on. Expect missing features, rough runtime
ergonomics, and breaking changes before a stable release.

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

## Why Try It

- Small, inspectable Rust implementation of the UPnP AV / DLNA MediaServer
  pieces needed for basic TV discovery, browsing, and playback.
- Sans-IO protocol crates keep parsing, encoding, and response planning separate
  from sockets, filesystems, and process-level runtime concerns.
- `no_std` protocol crates with `#![deny(unsafe_code)]` where the design allows
  it.
- HTTP media serving supports `HEAD`, full-file `GET`, byte ranges, and DLNA
  response headers needed by typical media clients.
- Initial real-device validation has focused on Sony TV discovery, browsing,
  and MP4 playback.

## Quick Start

Run the MediaServer against a media directory:

```sh
RUSTY_CASTLE_HOST=192.168.1.10 cargo run -p rusty-castle -- /path/to/media
```

Check the embedded build version:

```sh
cargo run -p rusty-castle -- --version
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

To capture HTTP interactions from a real DLNA client as replayable compatibility
scenario data, set `RUSTY_CASTLE_CAPTURE` to a JSON Lines output path:

```sh
RUSTY_CASTLE_CAPTURE=/tmp/sony-bravia-browse.jsonl \
  RUSTY_CASTLE_HOST=192.168.1.10 \
  cargo run -p rusty-castle -- /path/to/media
```

The capture file records request and response metadata for device description,
SCPD, SOAP, and media HTTP requests. Text and XML response bodies are included;
binary media bodies are summarized by byte length so fixtures stay small. This
scenario capture is separate from telemetry tracing such as OpenTelemetry.

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

The runtime recursively scans the media directory and exposes subdirectories as
ContentDirectory containers, so DLNA clients can browse the same folder
hierarchy. Richer metadata extraction, subtitles, transcoding, DVD menu support,
and AVTransport are still deferred.

## Benchmarks and Validation

`rusty-castle` includes lightweight `cargo bench` targets for hot protocol
paths that can be measured without live network devices or media files. These
benches use stable Rust with custom bench binaries, so they work with the
repository toolchain instead of requiring nightly `#[bench]`.

Run all current benchmarks:

```sh
cargo make bench
```

Or run individual benches:

```sh
cargo bench -p ssdp-core --bench parse
cargo bench -p media-http --bench range
```

Current benchmark coverage:

- SSDP `M-SEARCH` and `NOTIFY ssdp:alive` datagram parsing.
- HTTP `Range` header parsing.
- DLNA media partial-response planning.

Current performance-oriented properties:

- SSDP parsing borrows from the input datagram instead of allocating owned
  message data.
- Protocol encoders write into caller-provided buffers.
- Media HTTP response planning computes range metadata separately from file I/O,
  so tests can cover byte-range behavior without serving large files.
- The runtime streams file bodies from disk and supports byte-range reads for
  clients that probe or resume media playback.

Current validation before release:

```sh
cargo make ci
cargo test -p media-http --no-default-features
cargo test -p upnp-core --no-default-features
cargo test -p dlna-core --no-default-features
cargo test -p upnp-av-core --no-default-features
```

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

Build the Docker image:

```sh
cargo make docker-image
```

By default, the local image is tagged from the current version. Exact `vX.Y.Z`
git tags build `rusty-castle:X.Y.Z`; other commits build
`rusty-castle:X.Y.Z-dev-g<commit>`.

Override the image name or registry when needed:

```sh
RUSTY_CASTLE_IMAGE=registry.example.com/rusty-castle:dev cargo make docker-image
```

Run the container with a reachable LAN host address and mounted media
directory:

```sh
docker run --rm --network host \
  -e RUSTY_CASTLE_HOST=192.168.1.10 \
  -v /path/to/media:/media:ro \
  rusty-castle:0.0.1-dev-g<commit>
```

## Releases

Releases are driven by SemVer git tags with a `v` prefix, for example
`v0.0.1`. The release workflow requires the tag version to match
`crates/rusty-castle/Cargo.toml`.

For normal releases, use the developer release script:

```sh
scripts/release.sh
```

The script asks whether the release is a `patch`, `minor`, `major`, or custom
`X.Y.Z` version, shows the planned version and tag, runs tests, creates the
release commit and annotated tag, then pushes both to `origin`. You can also
pass the bump directly:

```sh
scripts/release.sh patch
scripts/release.sh minor
scripts/release.sh major
scripts/release.sh 0.0.1
```

Pushing the `vX.Y.Z` tag creates a GitHub Release with a Linux binary archive
and publishes a Docker image to GitHub Container Registry:

```text
ghcr.io/<owner>/<repo>:0.0.1
ghcr.io/<owner>/<repo>:sha-<commit>
```

Pushes to `main` publish development images that include the package version and
short commit hash:

```text
ghcr.io/<owner>/<repo>:0.0.1-dev-g<commit>
ghcr.io/<owner>/<repo>:sha-<commit>
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

## License

Licensed under the MIT License. See [LICENSE](LICENSE).

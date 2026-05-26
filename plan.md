# rusty-castle

A Rust-based UPnP AV / DLNA MediaServer implementation designed around sans-io and no_std principles.
Goals:
- Stream media to Sony TVs and other DLNA clients
- Keep protocol layers as sans-io / no_std whenever possible
- Clean separation between protocol logic and IO/runtime layers
- Support future embedded and runtime-agnostic use cases
- Organize reusable protocol crates
---

# Repository Structure
```text
rusty-castle/
  Cargo.toml
  crates/
    ssdp-core/
    upnp-core/
    upnp-av-core/
    dlna-core/
    media-http/
    rusty-castle/

⸻

Architecture

+-----------------------------+
|        rusty-castle         |
|        application          |
+-----------------------------+
               |
+-----------------------------+
|         media-http          |
|  hyper/axum/tokio adapter   |
+-----------------------------+
               |
+-----------------------------+
|         upnp-av-core        |
| ContentDirectory / DIDL     |
+-----------------------------+
               |
+-----------------------------+
|          upnp-core          |
| SOAP / Device / SCPD        |
+-----------------------------+
               |
+-----------------------------+
|          ssdp-core          |
|      SSDP sans-io           |
+-----------------------------+

⸻

Design Principles

sans-io

Protocol layers should not perform IO directly.

Example:

pub fn parse_soap_action(
    input: &[u8],
) -> Result<ActionRequest<'_>, SoapError>;
pub fn handle_browse(
    req: BrowseRequest<'_>,
    catalog: &impl MediaCatalog,
) -> Result<BrowseResponse, BrowseError>;

⸻

no_std

Prefer:

#![no_std]
extern crate alloc;

Keep std dependencies isolated to adapter/runtime layers.

⸻

Layer Separation

no_std / sans-io Layers

* SSDP parser/builder
* SOAP parser/builder
* DIDL-Lite builder
* protocolInfo builder
* Range parser
* UPnP action model
* ContentDirectory logic

std Layers

* UDP sockets
* HTTP servers
* filesystem access
* DVD access
* transcoding
* async runtimes

⸻

Crates

⸻

crates/ssdp-core

Existing implementation to be migrated into the workspace.

Responsibility

* SSDP parser
* SSDP builder
* M-SEARCH
* NOTIFY
* header model

Goal

- no_std
- optional alloc
- zero-copy parser

Tasks

* migrate existing repository
* integrate into workspace
* organize crate features
* add examples
* add interoperability tests

⸻

crates/upnp-core

Generic UPnP layer.

Responsibility

* DeviceDescription XML
* SCPD XML
* SOAP parser/builder
* UPnP errors
* device/service identifiers

Types

DeviceType
ServiceType
Udn
Usn
ActionRequest
ActionResponse

XML

Device Description

<device>

SCPD

<scpd>

SOAP

<s:Envelope>

Tasks

* XML writer abstraction
* device.xml builder
* SCPD builder
* SOAP parser
* SOAP response builder
* SOAP fault builder

⸻

crates/upnp-av-core

UPnP AV implementation.

This is the most important crate.

Responsibility

* ContentDirectory
* ConnectionManager
* DIDL-Lite
* Browse logic

⸻

ContentDirectory

Actions

* Browse
* GetSystemUpdateID
* GetSearchCapabilities
* GetSortCapabilities

BrowseRequest

pub struct BrowseRequest<'a> {
    pub object_id: &'a str,
    pub flag: BrowseFlag,
    pub filter: &'a str,
    pub starting_index: u32,
    pub requested_count: u32,
    pub sort_criteria: &'a str,
}

BrowseResponse

pub struct BrowseResponse<T> {
    pub result: T,
    pub number_returned: u32,
    pub total_matches: u32,
    pub update_id: u32,
}

⸻

DIDL-Lite

Support

* container
* item
* dc:title
* upnp:class
* res

Example

<DIDL-Lite>

⸻

ConnectionManager

Actions

* GetProtocolInfo
* GetCurrentConnectionIDs
* GetCurrentConnectionInfo

Tasks

* DIDL builder
* Browse implementation
* ContentDirectory serializer
* ConnectionManager implementation
* interoperability tests

⸻

crates/dlna-core

DLNA compatibility layer.

Responsibility

* protocolInfo
* DLNA flags
* media profile helpers

Example

http-get:*:video/mp4:DLNA.ORG_OP=01

Types

DlnaProfile
ProtocolInfo

Tasks

* protocolInfo builder
* DLNA flag builder
* MIME helpers
* Sony TV compatibility profiles

⸻

crates/media-http

HTTP adapter layer.

Responsibility

* GET
* HEAD
* Range
* static media serving

Required Headers

Accept-Ranges
Content-Range
transferMode.dlna.org
contentFeatures.dlna.org

Tasks

* HTTP server
* Range support
* media streaming
* compatibility tuning

⸻

crates/rusty-castle

Application crate.

Responsibility

* configuration
* filesystem scan
* catalog management
* runtime integration

Initial Scope

/media/{id}
/device.xml
/ContentDirectory/control

Tasks

* SSDP integration
* device.xml route
* SOAP route
* media route
* filesystem catalog
* logging

⸻

Development Phases

⸻

Phase 0

Goal

Initialize workspace.

Tasks

* initialize workspace
* migrate ssdp-core

⸻

Phase 1

Goal

UPnP Device and SOAP support.

Tasks

* device.xml
* SCPD
* SOAP

⸻

Phase 2

Goal

Implement ContentDirectory.

Tasks

* Browse
* DIDL-Lite
* static response

⸻

Phase 3

Goal

DLNA compatibility.

Tasks

* protocolInfo
* DLNA headers

⸻

Phase 4

Goal

HTTP streaming.

Tasks

* GET
* HEAD
* Range

⸻

Phase 5

Goal

Sony TV playback.

Tasks

* TV discovery
* media listing
* mp4 playback

⸻

Initial Compatibility Target

First Supported Media

* mp4
* mpg
* vob

Deferred Features

* DVD menu support
* ISO navigation
* transcoding
* subtitles
* AVTransport

⸻

Long-term Ideas

* async runtime abstraction
* embedded support
* renderer support
* transcoding pipeline
* subtitle support
* live streaming
* web UI
* gstreamer backend
* ffmpeg backend

⸻

Milestones

Milestone	Description
M1	SSDP integration
M2	device.xml
M3	SOAP
M4	Browse
M5	DIDL-Lite
M6	HTTP Range
M7	Sony TV playback (Sony TV discovery, browsing, media list, and MP4 playback validated)
M8	filesystem catalog
M9	DVD support

⸻

Success Criteria

Sony TV discovers the MediaServer
→ Browse works correctly
→ Media list is visible
→ mp4 playback works
→ vob playback works
→ Seeking works correctly

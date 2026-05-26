//! Static media catalog used by the initial application core.

use dlna_core::ProtocolInfoRef;

/// A media item exposed through ContentDirectory and `/media/{id}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaItem {
    /// Stable object id.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Public URL for the resource.
    pub url: String,
    /// MIME type.
    pub mime_type: String,
    /// Optional byte size.
    pub size: Option<u64>,
    /// Optional DLNA duration text.
    pub duration: Option<String>,
    /// DLNA protocol info.
    pub protocol_info: ProtocolInfoRef<'static>,
}

impl MediaItem {
    /// Creates an MP4 media item with Sony-oriented DLNA protocolInfo.
    pub fn mp4(id: impl Into<String>, title: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            url: url.into(),
            mime_type: "video/mp4".into(),
            size: None,
            duration: None,
            protocol_info: ProtocolInfoRef::sony_mp4(),
        }
    }
}

/// A simple in-memory catalog.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StaticCatalog {
    items: Vec<MediaItem>,
    update_id: u32,
}

impl StaticCatalog {
    /// Creates an empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a catalog from an existing item list.
    pub fn from_items(items: Vec<MediaItem>) -> Self {
        Self {
            items,
            update_id: 1,
        }
    }

    /// Adds an item and bumps the update id.
    pub fn push(&mut self, item: MediaItem) {
        self.items.push(item);
        self.update_id = self.update_id.wrapping_add(1).max(1);
    }

    /// Returns all catalog items.
    pub fn items(&self) -> &[MediaItem] {
        &self.items
    }

    /// Returns a catalog item by object id.
    pub fn item(&self, id: &str) -> Option<&MediaItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Returns the current ContentDirectory update id.
    pub fn update_id(&self) -> u32 {
        self.update_id
    }
}

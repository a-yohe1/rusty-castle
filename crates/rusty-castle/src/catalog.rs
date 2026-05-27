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
        let update_id = catalog_update_id(&items);
        Self { items, update_id }
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

fn catalog_update_id(items: &[MediaItem]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for item in items {
        hash_field(&mut hash, &item.id);
        hash_field(&mut hash, &item.title);
        hash_field(&mut hash, &item.url);
        hash_field(&mut hash, &item.mime_type);
        if let Some(size) = item.size {
            hash_field(&mut hash, &size.to_string());
        }
        if let Some(duration) = &item.duration {
            hash_field(&mut hash, duration);
        }
    }
    hash.max(1)
}

fn hash_field(hash: &mut u32, value: &str) {
    for byte in value.as_bytes() {
        *hash ^= u32::from(*byte);
        *hash = hash.wrapping_mul(0x0100_0193);
    }
    *hash ^= u32::from(0xffu8);
    *hash = hash.wrapping_mul(0x0100_0193);
}

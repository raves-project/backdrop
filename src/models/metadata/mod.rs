pub mod types;

use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use types::{Filesize, Format, Framerate, Resolution};

/// A media file's metadata. Common metadata is always present, while the `other`
/// field represents that which isn't standard in a dictionary (string, string)
/// form.
#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
    pub path: PathBuf,
    pub resolution: Resolution,
    pub filesize: Filesize,
    /// The MIME type for the media file.
    pub format: Format,
    pub creation_date: Option<SystemTime>,
    pub modified_date: Option<SystemTime>,
    /// When Raves first saw this file.
    pub first_seen_date: SystemTime,
    /// Any kind-specific metadata (e.g. video framerate, etc.)
    pub specific: SpecificMetadata,
    /// Metadata that isn't immensely common, but can be read as a string.
    pub other: Option<OtherMetadataMap>,
}

/// Metadata "specific" to one type of media.
#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub enum SpecificMetadata {
    #[non_exhaustive]
    Image {},

    #[non_exhaustive]
    AnimatedImage {
        frame_count: u32,
        framerate: Framerate,
    },

    #[non_exhaustive]
    Video { length: f64 },
}

#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct OtherMetadataValue {
    pub user_facing_name: Option<String>,
    pub value: String,
}

/// A representation for uncommon metadata that can only be read.
///
/// Also, it's a `HashMap` newtype to get around the lack of `PartialOrd`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OtherMetadataMap {
    pub internal_hashmap: HashMap<String, OtherMetadataValue>,
}

impl PartialOrd for OtherMetadataMap {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.internal_hashmap
            .len()
            .partial_cmp(&other.internal_hashmap.len())
    }
}

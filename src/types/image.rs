use serde::{Deserialize, Serialize};

use super::{
    metadata::{FileName, FileSize, Metadata, Resolution},
    tag::TagIdent,
    Media,
};

/// General forms of metadata found on an image. These are the searchable
/// kinds.
///
/// You may wish to see `EtcMetadata` for less common fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageMetadata {
    resolution: Resolution,
    file_size: FileSize,
    file_name: FileName,
    tags: Vec<TagIdent>,
}

pub type Image = Media<ImageMetadata>;

impl Image {}

impl Metadata for ImageMetadata {}

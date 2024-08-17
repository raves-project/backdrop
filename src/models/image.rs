use super::{
    metadata::{FileName, FileSize, Metadata, Resolution},
    tags::TagIdent,
    Media,
};

/// General forms of metadata found on an image. These are the searchable
/// kinds.
///
/// You may wish to see `EtcMetadata` for less common fields.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageMetadata {
    resolution: Resolution,
    file_size: FileSize,
    file_name: FileName,
    tags: Vec<TagIdent>,
}

impl Metadata for ImageMetadata {}

pub type Image = Media<ImageMetadata>;

impl Image {}

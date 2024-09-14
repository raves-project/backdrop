//! A builder for basic metadata using the `image` crate.
//!
//! Note that this is effectively a fallback for when there is no other metadata
//! available.

use std::path::Path;

use image::GenericImageView as _;

use crate::{
    error::RavesError,
    models::metadata::{
        builder::get_video_len,
        types::{Format, MediaKind, Resolution},
        SpecificMetadata,
    },
};

use super::MetadataBuilder;

impl MetadataBuilder {
    /// Applies EXIF data from `image` to `self`.
    #[tracing::instrument(skip(self))]
    pub(super) async fn apply_image(
        &mut self,
        path: &Path,
        format: Format,
    ) -> Result<(), RavesError> {
        // read the image into a buffer and grab its dimensions
        let img =
            image::open(path).map_err(|e| RavesError::ImageError(path.display().to_string(), e))?;
        let (width, height) = img.dimensions();
        tracing::debug!("got image dimensions from image crate: {width}x{height}");

        // apply format
        let media_kind = format.media_kind();
        self.format = Some(format);

        // resolution
        self.resolution = Some(Resolution::new(width, height));
        tracing::debug!("got resolution from image!");

        // specific
        self.specific = Some(match media_kind {
            MediaKind::Photo => SpecificMetadata::Image {},
            MediaKind::Video => {
                tracing::warn!("video detected, but the image crate doesn't handle videos!");
                get_video_len(path)?
            }
            MediaKind::AnimatedPhoto => unimplemented!(),
        });

        Ok(())
    }
}

//! A builder for basic metadata using the `image` crate.
//!
//! Note that this is effectively a fallback for when there is no other metadata
//! available.

use camino::Utf8Path;
use image::GenericImageView as _;
use sqlx::types::Json;

use crate::{
    error::RavesError,
    models::metadata::{builder::get_video_len, Format, MediaKind, SpecificMetadata},
};

use super::MediaBuilder;

impl MediaBuilder {
    /// Applies EXIF data from `image` to `self`.
    #[tracing::instrument(skip(self))]
    pub(super) async fn apply_image(
        &mut self,
        path: &Utf8Path,
        format: Format,
    ) -> Result<(), RavesError> {
        // read the image into a buffer and grab its dimensions
        let img = image::open(path).map_err(|e| RavesError::ImageError(path.to_string(), e))?;
        let (width, height) = img.dimensions();
        tracing::debug!("got image dimensions from image crate: {width}x{height}");

        // apply format
        let media_kind = format.media_kind();
        self.format = Some(Json(format));

        // resolution
        self.width_px = Some(width);
        self.height_px = Some(height);
        tracing::debug!("got resolution from image!");

        // specific
        self.specific_metadata = Some(Json(match media_kind {
            MediaKind::Photo => SpecificMetadata::Image {},
            MediaKind::Video => {
                tracing::warn!("video detected, but the image crate doesn't handle videos!");
                get_video_len(path)?
            }
            MediaKind::AnimatedPhoto => unimplemented!(),
        }));

        Ok(())
    }
}

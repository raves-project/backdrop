//! A builder for basic metadata using the `image` crate.
//!
//! Note that this is effectively a fallback for when there is no other metadata
//! available.

use camino::Utf8Path;
use image::GenericImageView as _;
use sqlx::types::Json;

use crate::{
    error::RavesError,
    models::media::metadata::{MediaKind, SpecificMetadata},
};

use super::MediaBuilder;

impl MediaBuilder {
    /// Applies EXIF data from `image` to `self`.
    #[tracing::instrument(skip(self))]
    pub(super) async fn apply_image(
        &mut self,
        path: &Utf8Path,
        media_kind: MediaKind,
    ) -> Result<(), RavesError> {
        // read the image into a buffer and grab its dimensions
        let img = image::open(path).map_err(|e| RavesError::ImageError(path.to_string(), e))?;
        let (width, height) = img.dimensions();
        tracing::debug!("got image dimensions from image crate: {width}x{height}");

        // resolution
        self.width_px = Some(width);
        self.height_px = Some(height);
        tracing::debug!("got resolution from image!");

        // specific
        if media_kind == MediaKind::Photo {
            self.specific_metadata = Some(Json(SpecificMetadata::Image {}))
        }

        Ok(())
    }
}

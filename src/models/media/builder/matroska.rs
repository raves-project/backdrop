//! A builder for basic metadata using the `matroska` crate.
//!
//! Note that this is effectively a fallback for when there is no other metadata
//! available.

use crate::{
    error::RavesError,
    models::media::metadata::{Format, MediaKind, SpecificMetadata},
};

use camino::Utf8Path;
use matroska::Settings;
use sqlx::types::Json;

use super::MediaBuilder;

impl MediaBuilder {
    /// Applies Matroska data from `matroska` to `self`.
    #[tracing::instrument(skip(self))]
    pub(super) async fn apply_matroska(
        &mut self,
        path: &Utf8Path,
        format: Format,
    ) -> Result<(), RavesError> {
        if format.media_kind() == MediaKind::Video {
            let mkv =
                matroska::open(path).map_err(|e| RavesError::MatroskaError(path.to_string(), e))?;

            let vt = mkv
                .video_tracks()
                .next()
                .ok_or(RavesError::FileMissingMetadata(
                    path.to_string(),
                    "no video track".into(),
                ))?;

            // resolution
            if let Settings::Video(v) = &vt.settings {
                self.width_px = Some(v.pixel_width as u32);
                self.height_px = Some(v.pixel_height as u32);
                tracing::debug!(
                    "got resolution from matroska: width {}, height {}",
                    v.pixel_width,
                    v.pixel_height
                );
            }

            // specific
            if let Some(duration) = mkv.info.duration {
                self.specific_metadata = Some(Json(SpecificMetadata::Video {
                    length: duration.as_secs_f64(),
                }));
                tracing::debug!(
                    "got video duration from matroska: length {}",
                    duration.as_secs_f64()
                );
            }
        }

        Ok(())
    }
}

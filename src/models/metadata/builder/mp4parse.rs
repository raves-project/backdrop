//! A builder for basic metadata using the `mp4parse` crate.
//!
//! Note that this is effectively a fallback for when there is no other metadata
//! available.

use camino::Utf8Path;
use sqlx::types::Json;

use crate::{
    error::RavesError,
    models::metadata::{
        builder::get_video_len,
        types::{Format, MediaKind},
    },
};

use super::MediaBuilder;

impl MediaBuilder {
    /// Applies EXIF data from `mp4parse` to `self`.
    #[tracing::instrument(skip(self))]
    pub(super) async fn apply_mp4parse(
        &mut self,
        path: &Utf8Path,
        format: Format,
    ) -> Result<(), RavesError> {
        // check if it's a video
        if format.media_kind() == MediaKind::Video {
            let mut f = std::fs::File::open(path).map_err(|e| RavesError::FileMetadataFailure {
                path: path.to_string(),
                err: e,
            })?;

            // read the file
            let info = tokio::task::spawn_blocking(move || mp4parse::read_mp4(&mut f))
                .await?
                .map_err(|e| RavesError::Mp4parseError(path.to_string(), e))?;

            // grab first video track
            if let Some(track) = info
                .tracks
                .iter()
                .find(|t| t.track_type == mp4parse::TrackType::Video)
            {
                let header = track.tkhd.clone().ok_or(RavesError::FileMissingMetadata(
                    path.to_string(),
                    "no track header".into(),
                ))?;

                // resolution
                self.width_px = Some(header.width);
                self.height_px = Some(header.height);
                tracing::debug!(
                    "got resolution from mp4parse! ({} x {})",
                    header.width,
                    header.height
                );

                // specific
                self.specific_metadata = Some(Json(get_video_len(path)?));

                // other
                if let Some(Ok(userdata)) = info.userdata {
                    if let Some(_meta) = userdata.meta {
                        // TODO: you can get more info here.
                        // ...just have to manually add 50 fields lmao
                    }
                }
            }
        }

        Ok(())
    }
}

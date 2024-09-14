//! A builder for basic metadata using the `mp4parse` crate.
//!
//! Note that this is effectively a fallback for when there is no other metadata
//! available.

use std::path::Path;

use crate::{
    error::RavesError,
    models::metadata::{
        builder::get_video_len,
        types::{Format, MediaKind, Resolution},
    },
};

use super::MetadataBuilder;

impl MetadataBuilder {
    /// Applies EXIF data from `mp4parse` to `self`.
    #[tracing::instrument(skip(self))]
    pub(super) async fn apply_mp4parse(
        &mut self,
        path: &Path,
        format: Format,
    ) -> Result<(), RavesError> {
        // check if it's a video
        if format.media_kind() == MediaKind::Video {
            let mut f = std::fs::File::open(path).map_err(|e| RavesError::FileMetadataFailure {
                path: path.display().to_string(),
                err: e,
            })?;

            // read the file
            let info = tokio::task::spawn_blocking(move || mp4parse::read_mp4(&mut f))
                .await?
                .map_err(|e| RavesError::Mp4parseError(path.display().to_string(), e))?;

            // grab first video track
            if let Some(track) = info
                .tracks
                .iter()
                .find(|t| t.track_type == mp4parse::TrackType::Video)
            {
                let header = track.tkhd.clone().ok_or(RavesError::FileMissingMetadata(
                    path.display().to_string(),
                    "no track header".into(),
                ))?;

                // resolution
                self.resolution = Some(Resolution::new(header.width, header.height));
                tracing::debug!("got resolution from mp4parse!");

                // specific
                self.specific = Some(get_video_len(path)?);

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

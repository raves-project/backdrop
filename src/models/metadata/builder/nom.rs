use camino::Utf8Path;
use nom_exif::{parse_exif_async, Exif as NomExif, ExifIter, ExifTag};
use sqlx::types::Json;
use tokio::task::spawn_blocking;
use tokio::try_join;

use crate::{
    error::RavesError,
    models::metadata::{
        builder::get_video_len,
        types::{Format, MediaKind},
        OtherMetadataMap, OtherMetadataValue, SpecificMetadata,
    },
};

use super::MediaBuilder;

impl MediaBuilder {
    /// Applies EXIF data from `nom_exif` to `self`.
    #[tracing::instrument(skip(self))]
    pub(super) async fn apply_nom_exif(
        &mut self,
        path: &Utf8Path,
        format: Format,
    ) -> Result<(), RavesError> {
        tracing::debug!("grabbing exif data...");
        let (_, (iter, exif)) = try_join! {
            self.file(path),
            fut(path),
        }?;
        tracing::debug!("got exif data!");

        let media_kind = format.media_kind();

        // look for cool shit in the exif
        // res
        let w = exif
            .get(ExifTag::ImageWidth)
            .ok_or(RavesError::FileMissingMetadata(
                path.to_string(),
                "no width".into(),
            ))?
            .as_u32()
            .ok_or(RavesError::FileMissingMetadata(
                path.to_string(),
                "no width".into(),
            ))?;
        let h = exif
            .get(ExifTag::ImageHeight)
            .ok_or(RavesError::FileMissingMetadata(
                path.to_string(),
                "no height".into(),
            ))?
            .as_u32()
            .ok_or(RavesError::FileMissingMetadata(
                path.to_string(),
                "no width".into(),
            ))?;

        self.width_px = Some(w);
        self.height_px = Some(h);
        tracing::debug!("got resolution from exif!");

        // specific
        self.specific_metadata = Some(Json(match media_kind {
            MediaKind::Photo => SpecificMetadata::Image {},
            MediaKind::Video => get_video_len(path)?,
            MediaKind::AnimatedPhoto => unimplemented!(),
        }));
        tracing::debug!("got specific metadata from exif!");

        // other
        let mut mapped = OtherMetadataMap::new();
        for entry in iter {
            // FIXME: find some way to convert all these tags
            let key = entry.tag_code().to_string(); // FIXME: this is a number
            let Some(value) = entry.take_value() else {
                continue;
            };

            let value = OtherMetadataValue {
                user_facing_name: None, // TODO
                value: value.to_string(),
            };

            mapped.0.insert(key, value);
        }
        self.other_metadata = Some(Json(mapped));
        tracing::debug!("got other metadata from exif!");

        tracing::debug!("finished looking for exif data!");

        Ok(())
    }
}

async fn fut(path: &Utf8Path) -> Result<(ExifIter, NomExif), RavesError> {
    let path_str = path.to_string();

    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|e| RavesError::FileMetadataFailure {
            path: path_str.clone(),
            err: e,
        })?;
    tracing::debug!("opened file!");

    let iter = parse_exif_async(file, None)
        .await
        .map_err(|e| RavesError::NomExifError(path_str.clone(), e))?
        .ok_or_else(|| RavesError::FileMissingMetadata(path_str.clone(), "no exif data".into()))?;
    tracing::debug!("parsed exif data!");

    // convert
    spawn_blocking(move || (iter.clone(), iter.into()))
        .await
        .map_err(RavesError::TokioJoinError)
}

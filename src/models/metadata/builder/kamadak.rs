use camino::Utf8Path;
use kamadak_exif::{Exif as KamadakExif, In, Tag};
use sqlx::types::Json;
use tokio::try_join;

use crate::{
    error::RavesError,
    models::metadata::{
        builder::get_video_len, Format, MediaKind, OtherMetadataMap, OtherMetadataValue,
        SpecificMetadata,
    },
};

use super::MediaBuilder;

impl MediaBuilder {
    /// Applies EXIF data from `kamadak_exif` to `self`.
    #[tracing::instrument(skip(self))]
    pub(super) async fn apply_kamadak_exif(
        &mut self,
        path: &Utf8Path,
        format: Format,
    ) -> Result<(), RavesError> {
        let (_, exif) = try_join! {
            self.file(path),
            look(path),
        }?;
        tracing::debug!("got exif data from kamadak-exif!");

        let p = In::PRIMARY;
        let err = |msg: &str| {
            tracing::error!("Error while building metadata with `kamadak-exif`. err: {msg}");
            RavesError::FileMissingMetadata(path.to_string(), msg.to_string())
        };
        tracing::debug!("looking for exif data...");

        // resolution
        let kamadak_exif::Value::Long(ref w) = exif
            .get_field(Tag::PixelXDimension, p)
            .ok_or(err("no width"))?
            .value
        else {
            return Err(err("no width"));
        };
        let kamadak_exif::Value::Long(ref h) = exif
            .get_field(Tag::PixelYDimension, p)
            .ok_or(err("no height"))?
            .value
        else {
            return Err(err("no height"));
        };

        self.width_px = Some(*w.first().ok_or(err("no width"))?);
        self.height_px = Some(*h.first().ok_or(err("no width"))?);
        tracing::debug!("got resolution from exif!");

        // specific
        self.specific_metadata = Some(Json(match format.media_kind() {
            MediaKind::Photo => SpecificMetadata::Image {},
            MediaKind::Video => get_video_len(path)?,
            MediaKind::AnimatedPhoto => unimplemented!(),
        }));
        tracing::debug!("got specific metadata from exif!");

        // other
        let mut mapped = OtherMetadataMap::new();
        for field in exif.fields() {
            let key = field.tag.to_string();
            let value = OtherMetadataValue {
                user_facing_name: Some(key.clone()),
                value: field.display_value().to_string(),
            };

            mapped.0.insert(key, value);
        }
        self.other_metadata = Some(Json(mapped));
        tracing::debug!("got other metadata from exif!");

        tracing::debug!("finished looking for exif data!");

        Ok(())
    }
}

/// We use this function to 'look' at the metadata of the file, returning EXIF
/// information from `kamadak_exif`.
///
/// This is `async` as we use `tokio` to grab a file handle, then spawn a task
/// to process it synchronously, awaiting its completion.
async fn look(path: &Utf8Path) -> Result<KamadakExif, RavesError> {
    let path = path.to_path_buf(); // extends lifetime by copying data

    // grab the file with tokio (avoid blocking)
    let file = tokio::fs::File::open(path.to_path_buf())
        .await
        .inspect_err(|e| tracing::warn!("Failed to open file for `kamadak_exif`! err: {e}"))
        .map_err(|e| RavesError::FileMetadataFailure {
            path: path.clone().into(),
            err: e,
        })?
        .into_std()
        .await;

    // make a buffer where we'll read the file
    let mut buf_reader = std::io::BufReader::new(file);
    let exif_reader = kamadak_exif::Reader::new();

    // hand that off to `tokio`
    tokio::task::spawn_blocking(move || -> Result<KamadakExif, RavesError> {
        exif_reader
            .read_from_container(&mut buf_reader)
            .inspect_err(|e| tracing::warn!("`kamadak-exif` failed to get metadata. err: {e}"))
            .map_err(|e| RavesError::KamadakExifError(path.to_string(), e))
    })
    .await
    .map_err(RavesError::TokioJoinError)?
}

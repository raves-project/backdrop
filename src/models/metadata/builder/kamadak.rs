use std::path::Path;

use kamadak_exif::{Exif as KamadakExif, In, Tag};
use tokio::try_join;

use crate::{
    error::RavesError,
    models::metadata::{
        builder::get_video_len,
        types::{Format, MediaKind, Resolution},
        OtherMetadataMap, OtherMetadataValue, SpecificMetadata,
    },
};

use super::MetadataBuilder;

impl MetadataBuilder {
    /// Applies EXIF data from `kamadak_exif` to `self`.
    #[tracing::instrument(skip(self))]
    pub(super) async fn apply_kamadak_exif(
        &mut self,
        path: &Path,
        format: Format,
    ) -> Result<(), RavesError> {
        let (_, exif) = try_join! {
            self.file(path),
            look(path),
        }?;

        tracing::debug!("got exif data from kamadak-exif!");

        let p = In::PRIMARY;

        let err = |msg: &str| {
            RavesError::FileMissingMetadata(path.display().to_string(), msg.to_string())
        };

        tracing::debug!("looking for exif data...");

        // resolution
        let kamadak_exif::Value::Long(ref w) = exif
            .get_field(Tag::ImageWidth, p)
            .ok_or(err("no width"))?
            .value
        else {
            return Err(err("no width"));
        };
        let kamadak_exif::Value::Long(ref h) = exif
            .get_field(Tag::ImageLength, p)
            .ok_or(err("no height"))?
            .value
        else {
            return Err(err("no height"));
        };

        self.resolution = Some(Resolution::new(
            *w.first().ok_or(err("no width"))?,
            *h.first().ok_or(err("no height"))?,
        ));
        tracing::debug!("got resolution from exif!");

        // specific
        self.specific = Some(match format.media_kind() {
            MediaKind::Photo => SpecificMetadata::Image {},
            MediaKind::Video => get_video_len(path)?,
            MediaKind::AnimatedPhoto => unimplemented!(),
        });
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
        self.other = Some(Some(mapped));
        tracing::debug!("got other metadata from exif!");

        tracing::debug!("finished looking for exif data!");

        Ok(())
    }
}

// to get rid of that god-forsaken `JoinError`
async fn look(path: &Path) -> Result<KamadakExif, RavesError> {
    let path = path.to_path_buf();
    let path_str = path.display().to_string();

    tokio::task::spawn_blocking(|| -> Result<KamadakExif, RavesError> {
        let mut file = std::fs::File::open(path).map_err(|_e| RavesError::MediaDoesntExist {
            path: path_str.clone(),
        })?;

        let mut buf_reader = std::io::BufReader::new(&mut file);
        let exif_reader = kamadak_exif::Reader::new();
        exif_reader
            .read_from_container(&mut buf_reader)
            .map_err(|e| RavesError::KamadakExifError(path_str, e))
    })
    .await
    .map_err(RavesError::TokioJoinError)?
}

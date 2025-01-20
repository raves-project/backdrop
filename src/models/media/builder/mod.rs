//! # Metadata Builders
//!
//! Assists in ascertaining metadata of different kinds in one format.
//!
//! Note that this should eventually be replaced with a fleshed-out library
//! with full support for all these types!

pub mod avif;
pub mod generic;
pub mod image_crate;
pub mod kamadak;
pub mod matroska;
pub mod mp4parse;
pub mod nom;

use camino::Utf8Path;
use chrono::{DateTime, Utc};
use sqlx::types::Json;
use uuid::Uuid;

use crate::{
    database::{DATABASE, HASHES_TABLE, INFO_TABLE},
    error::RavesError,
    models::{
        media::{metadata::MediaKind, Media},
        tags::Tag,
    },
};

use super::{
    hash::MediaHash,
    metadata::{Format, OtherMetadataMap, SpecificMetadata},
};

/// A media file's metadata. Common metadata is always present, while the `other`
/// field represents that which isn't standard in a dictionary (string, string)
/// form.
#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct MediaBuilder {
    /// How large the file is, in bytes.
    pub filesize: Option<i64>,

    /// The MIME type (format) of the file.
    pub format: Option<Json<Format>>,

    /// The time the file was created, according to the file system.
    ///
    /// This could be inaccurate or missing depending on the file's source.
    pub creation_date: Option<DateTime<Utc>>,

    /// The time the file was last modified, according to the file system.
    ///
    /// Might be inaccurate or missing.
    pub modification_date: Option<DateTime<Utc>>,

    /// The media's width (horizontal) in pixels.
    pub width_px: Option<u32>,

    /// The media's height (vertical) in pixels.
    pub height_px: Option<u32>,

    /// Additional metadata that's specific to the media's kind, such as a
    /// video's framerate.
    pub specific_metadata: Option<Json<SpecificMetadata>>,

    /// Metadata that isn't immensely common, but can be read as a string.
    ///
    /// Or, in other words, it's a hashmap of data.
    ///
    /// This is stored as `Json` for the database.
    pub other_metadata: Option<Json<OtherMetadataMap>>,

    /// The tags of a media file. Note that these can come from the file's EXIF
    /// metadata or Rave's internals.
    pub tags: Json<Vec<Tag>>,
}

impl MediaBuilder {
    /// Constructs a [`Media`] file representation from this [`MediaBuilder`].
    #[tracing::instrument(skip(self))]
    pub(super) async fn build<P: AsRef<Utf8Path> + std::fmt::Debug>(
        self,
        path: P,
    ) -> Result<Media, RavesError> {
        let path = path.as_ref();
        self.build_internal(path).await
    }

    /// The internal 'build' function to create a [`Media`] from [`MediaBuilder`].
    /// This should only be called from [`MediaBuilder::build`].
    ///
    /// NOTE: `path` **must** be an absolute path that's been canonicalized.
    ///
    /// It has the following pipeline:
    ///
    /// 1. Grab format of `path`.
    /// 2. Apply it to self, but match on the contained `MediaKind` to better
    ///    determine next steps.
    /// 3. Based on our MediaKind...
    ///     - If we're a photo or animated photo,
    ///         - AVIF only: apply `avif_parse` crate
    ///         - TIFF/JPEG/HEIF/PNG/WebP: apply `kamadak_exif` crate
    ///         - anything: apply `image` crate
    ///     - If we're a video,
    ///         - MP4/MOV only: apply `nom_exif` crate
    ///         - MP4 only: apply `mp4parse` crate
    ///         - MOV/MKV/WebM: apply `matroska` crate
    /// 4. Check for a previous cache of the media.
    /// 5. If present, steal its UUID and first-seen datetime.
    /// 6. Unwrap all fields and stick into a new `Media`.
    /// 7. Return it.
    #[tracing::instrument(skip(self))]
    async fn build_internal(mut self, path: &Utf8Path) -> Result<Media, RavesError> {
        // before anything, let's make sure the media file has an album to use!
        let album = path
            .parent()
            .map(|p| p.to_path_buf().to_string())
            .inspect(|parent| {
                tracing::debug!("Found album (parent) for media file! path: {parent}")
            })
            .ok_or_else(|| {
                tracing::warn!("Given a supposed file path, but failed to find its parent!");
                RavesError::MediaFilePathNoParent(path.to_path_buf())
            })?;

        // grab format and apply it to self
        let format = format(path).await?;
        let mime_type = format.mime_type();
        let media_kind = format.media_kind();
        self.format = Some(Json(format));

        // grab file metadata real quick
        _ = self
            .file(path)
            .await
            .inspect_err(|e| tracing::warn!("Failed to get file metadata! err: {e}"));

        // based on the 'kind' of media we're dealing with, we'll choose different
        // libraries to apply to internal metadata
        match &media_kind {
            MediaKind::Photo | MediaKind::AnimatedPhoto => {
                // if we're avif, apply the avif crate
                let avif_result = if mime_type.to_lowercase().contains("avif") {
                    self.apply_avif(path, media_kind)
                        .await
                        .map_err(|e| tracing::warn!("Failed to parse with `avif_parse`. err: {e}"))
                } else {
                    Err(())
                };

                // really this is only for tiff/jpeg/heif/png/webp, but we can
                // parse everything since there's a lot of other not-well-known
                // file types between all those
                let kamadak_result = self
                    .apply_kamadak_exif(path, media_kind)
                    .await
                    .map_err(|e| tracing::debug!("Failed to aprse with `kamadak_exif`. err: {e}"));

                // finally, use the `image` crate when we're out of luck :p
                if avif_result.is_err() && kamadak_result.is_err() {
                    _ = self
                        .apply_image(path, media_kind)
                        .await
                        .map_err(|e| tracing::error!("Failed to parse with `image`! err: {e}"));
                }
            }

            MediaKind::Video => {
                // ffmpeg: get video length
                let specific_metadata = get_video_len(path)
                    .inspect_err(|e| tracing::error!("Failed to get video length. err: {e}"))?;

                self.specific_metadata = Some(Json(specific_metadata));

                // apply `mp4`
                _ = self.apply_mp4parse(path, media_kind).await.map_err(|e| {
                    tracing::debug!("Failed to parse with `mp4parse`. err: {e}");
                });

                // apply `matroska`
                _ = self.apply_matroska(path, media_kind).await.map_err(|e| {
                    tracing::debug!("Failed to parse with `matroska`. err: {e}");
                });

                // apply `nom_exif`
                _ = self.apply_nom_exif(path, media_kind).await;
            }
        }

        // grab the static fields
        let StaticFields {
            id,
            first_seen_date,
        } = get_static_fields(path).await?;

        Ok(Media {
            id,

            album: album.to_string(),
            path: path.to_string(),

            filesize: self.filesize.ok_or(RavesError::FileMissingMetadata(
                path.to_string(),
                "no file size given".into(),
            ))?,

            creation_date: self.creation_date,
            modification_date: self.modification_date,

            format: self.format.ok_or(RavesError::FileMissingMetadata(
                path.to_string(),
                "no format given".into(),
            ))?,
            width_px: self.width_px.ok_or(RavesError::FileMissingMetadata(
                path.to_string(),
                "no width (res) given".into(),
            ))?,
            height_px: self.height_px.ok_or(RavesError::FileMissingMetadata(
                path.to_string(),
                "no width (res) given".into(),
            ))?,
            specific_metadata: self
                .specific_metadata
                .ok_or(RavesError::FileMissingMetadata(
                    path.to_string(),
                    "no specific metadata (file kind variant)".into(),
                ))?,
            other_metadata: self.other_metadata,

            first_seen_date,

            tags: self.tags,
        })
    }
}

/// Grabs the format of the media file at `path`.
#[tracing::instrument]
async fn format(path: &Utf8Path) -> Result<Format, RavesError> {
    let path_str = path.to_string();

    // infer the MIME type for the file
    tracing::debug!("Grabbing MIME type...");
    let mime = infer::get_from_path(path)
        .map_err(|_e| RavesError::MediaDoesntExist {
            path: path_str.clone(),
        })?
        .ok_or(RavesError::NoMimeData {
            path: path_str.clone(),
        })?;

    // make the format
    tracing::debug!("Creating format from MIME...");
    let format = Format::new_from_mime(mime.mime_type())
        .ok_or(RavesError::FileNotSupportedMedia {
            path: path_str.clone(),
        })
        .inspect_err(|e| tracing::error!("Failed to create MIME type! err: {e}"))?;

    Ok(format)
}

/// Either steals or creates the static fields required to create a [`Media`].
#[tracing::instrument]
async fn get_static_fields(path: &Utf8Path) -> Result<StaticFields, RavesError> {
    // if the media was previously saved in the database, we'll need to use
    // its id and 'first seen date'
    let (id, first_seen_date) = 'a: {
        let mut conn = DATABASE.acquire().await.inspect_err(|e| {
            tracing::error!("Failed to connect to database in metadata builder! err: {e}")
        })?;

        // if we find our path in there, we can just use the old stuff
        let old_media_path_query =
            sqlx::query_as::<_, Media>(&format!("SELECT * FROM {INFO_TABLE} WHERE path = $1"))
                .bind(path.to_string())
                .fetch_optional(&mut *conn)
                .await
                .inspect_err(|e| tracing::error!("(path) Failed to query database! err: {e}"))?;

        if let Some(old_media) = old_media_path_query {
            break 'a (old_media.id, old_media.first_seen_date);
        }

        // we can also check for duplicate photos, as that's fair game for
        // 'first seen', though we'll also need to create a new UUID.
        if let Ok(hash) = MediaHash::hash_file(path).await {
            let old_media_hash_query = sqlx::query_as::<_, Media>(&format!(
                "SELECT * FROM {HASHES_TABLE} WHERE hash = $1"
            ))
            .bind(hash.as_bytes().to_vec())
            .fetch_optional(&mut *conn)
            .await
            .inspect_err(|e| tracing::error!("(hash) Failed to query database! err: {e}"))?;

            if let Some(old_media) = old_media_hash_query {
                break 'a (Uuid::new_v4(), old_media.first_seen_date);
            }
        }

        (Uuid::new_v4(), Utc::now())
    };

    Ok(StaticFields {
        id,
        first_seen_date,
    })
}

/// Fields that don't change across metadata generations.
struct StaticFields {
    id: Uuid,
    first_seen_date: DateTime<Utc>,
}

impl Default for MediaBuilder {
    fn default() -> Self {
        Self {
            filesize: None,
            format: None,
            creation_date: None,
            modification_date: None,
            width_px: None,
            height_px: None,
            specific_metadata: None,
            other_metadata: None,
            tags: Json(vec![]),
        }
    }
}

/// Grabs the video length of a media file using FFmpeg.
pub fn get_video_len(path: &Utf8Path) -> Result<SpecificMetadata, RavesError> {
    let path_str = path.to_string();

    // let's ask ffmpeg what it thinks
    tracing::trace!("video detected. asking ffmpeg to handle...");
    ffmpeg_next::init()?;
    let t = ffmpeg_next::format::input(path).map_err(|e| RavesError::FfmpegFailedProcessing {
        path: path_str.clone(),
        err: e.to_string(),
    })?;

    // grab the first video stream and see how long it is
    let video_length = t
        .streams()
        .find(|s| s.parameters().medium() == ffmpeg_next::media::Type::Video)
        .map(|s| (ffmpeg_next::Rational::new(s.duration() as i32, 1)) * s.time_base())
        .map(|s| s.0 as f64 / s.1 as f64)
        .unwrap_or(0_f64);
    tracing::trace!("video len is {video_length}.");

    Ok(SpecificMetadata::Video {
        length: video_length,
    })
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use camino::Utf8PathBuf;
    use chrono::{DateTime, Utc};
    use sqlx::types::Json;
    use uuid::Uuid;

    use crate::{
        database::{self, InsertIntoTable as _, DATABASE, INFO_TABLE},
        models::media::{
            metadata::{Format, SpecificMetadata},
            Media,
        },
    };

    use super::MediaBuilder;

    /// The `MediaBuilder` should keep the `id` and `first_seen_date` fields as-is.
    #[tokio::test]
    async fn media_builder_keeps_static_fields() {
        let temp_dir = TempDir::new().unwrap();
        // set up the db
        database::DB_FOLDER_PATH
            .set(Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap())
            .unwrap();

        let path = Utf8PathBuf::from("tests/assets/fear.avif")
            .canonicalize_utf8()
            .unwrap();

        // add a fake file to it
        let old_media = Media {
            id: Uuid::nil(),
            path: path.to_string(),
            album: "tests/assets".into(),
            filesize: 0,
            format: Json(Format::new_from_mime("image/avif").unwrap()),
            creation_date: None,
            modification_date: None,
            first_seen_date: DateTime::<Utc>::MIN_UTC,
            width_px: 32,
            height_px: 32,
            specific_metadata: Json(SpecificMetadata::Image {}),
            other_metadata: None,
            tags: Json(vec![]),
        };

        // insert into db
        let mut conn = DATABASE.acquire().await.unwrap();
        old_media
            .make_insertion_query()
            .execute(&mut *conn)
            .await
            .unwrap();

        // now run the media builder on a real file...
        let new_media = MediaBuilder::default().build(&path).await.unwrap();

        assert_eq!(old_media.id, new_media.id, "same uuids");
        assert_eq!(
            old_media.first_seen_date, new_media.first_seen_date,
            "same first seen dates"
        );

        // insert into the database and ensure they're still accurate
        new_media
            .make_insertion_query()
            .execute(&mut *conn)
            .await
            .unwrap();
        let inserted_new_media =
            sqlx::query_as::<_, Media>(&format!("SELECT * FROM {INFO_TABLE} LIMIT 1"))
                .fetch_one(&mut *conn)
                .await
                .unwrap();

        assert_eq!(
            old_media.id, inserted_new_media.id,
            "post-insert same uuids"
        );
        assert_eq!(
            old_media.first_seen_date, inserted_new_media.first_seen_date,
            "post-insert same first seen dates"
        );
    }
}

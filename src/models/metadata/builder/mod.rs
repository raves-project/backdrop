//! # Metadata Builders
//!
//! Assists in ascertaining metadata of different kinds in one format.
//!
//! Note that this should eventually be replaced with a fleshed-out library
//! with full support for all these types!

pub mod avif;
pub mod image_crate;
pub mod kamadak;
pub mod matroska;
pub mod mp4parse;
pub mod nom;

use std::os::unix::fs::MetadataExt;

use camino::Utf8Path;
use chrono::{DateTime, Utc};
use infer::{video::*, Type};
use sqlx::types::Json;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::{
    database::{DATABASE, INFO_TABLE},
    error::RavesError,
    models::{
        media::Media,
        metadata::{
            types::{Format, MediaKind},
            OtherMetadataMap, SpecificMetadata,
        },
        tags::Tag,
    },
};

/// A media file's metadata. Common metadata is always present, while the `other`
/// field represents that which isn't standard in a dictionary (string, string)
/// form.
#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct MediaBuilder {
    /// The last known file path for this media file.
    pub path: Option<String>,

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
    /// Returns metadata found from a file.
    #[tracing::instrument]
    pub async fn apply<P>(mut self, path: P) -> Result<Media, RavesError>
    where
        P: AsRef<Utf8Path> + std::fmt::Debug,
    {
        let path = path.as_ref();

        // get the format
        tracing::debug!("grabbing format...");
        let (format, _inferred) = Self::format(path).await?;

        // apply format
        tracing::debug!("applying format...");
        let media_kind = format.media_kind();
        self.format = Some(Json(format.clone()));

        tracing::debug!("applying metadata...");
        match media_kind {
            MediaKind::Photo => {
                // first, if we think it's an avif file, use the `avif-parse` crate!
                if format.mime_type().to_lowercase().contains("avif") {
                    let _avif = self.apply_avif(path, format.clone()).await;
                }

                // kamadak-exif has a lot of photo formats
                let _kamadak = self.apply_kamadak_exif(path, format.clone()).await;

                // fallback to image crate
                let _image = self.apply_image(path, format).await;

                return self.build().await;
            }
            MediaKind::Video => {
                // nom_exif supports mp4 and mov.
                // TODO: other crates for more formats?
                let nom = self.apply_nom_exif(path, format.clone()).await;
                if nom.is_ok() {
                    return self.build().await;
                }

                tracing::warn!("couldn't get metadata from nom_exif. using video fallbacks...");

                // let's read the first 38 bytes of the file.
                // that lets us check the actual container type
                let mut buf = [0; 38];
                tokio::fs::File::open(path)
                    .await
                    .map_err(|e| RavesError::FileMetadataFailure {
                        path: path.to_string(),
                        err: e,
                    })?
                    .read_exact(&mut buf)
                    .await
                    .map_err(|e| RavesError::FileMetadataFailure {
                        path: path.to_string(),
                        err: e,
                    })?;

                // use generic crates for exif-less containers
                if is_mp4(&buf) {
                    tracing::warn!("detected mp4 container. using mp4parse...");
                    self.apply_mp4parse(path, format).await?;
                } else if is_mov(&buf) || is_mkv(&buf) || is_webm(&buf) {
                    tracing::warn!("detected matroska container. using matroska crate...");
                    self.apply_matroska(path, format).await?;
                } else {
                    tracing::error!(
                        "an unsupported video container was detected. trying ffmpeg..."
                    );
                    unimplemented!()
                }
            }
            MediaKind::AnimatedPhoto => unimplemented!(),
        };

        tracing::debug!("finished applying metadata!");
        self.build().await
    }
}

// private methods
impl MediaBuilder {
    /// Adds typical file attributes to `self`.
    #[tracing::instrument(skip(self))]
    async fn file(&mut self, path: &Utf8Path) -> Result<(), RavesError> {
        let path_str = path.to_string();

        // err if the file doesn't open
        let metadata = tokio::fs::metadata(path)
            .await
            .inspect_err(|e| tracing::warn!("Failed to open file for metadata. err: {e}"))
            .map_err(|_e| RavesError::MediaDoesntExist { path: path_str })?;
        tracing::debug!("got file metadata!");

        self.path = Some(path.to_string());
        self.filesize = Some(metadata.size() as i64);
        self.creation_date = metadata.created().ok().map(|st| st.into());
        self.modification_date = metadata.modified().ok().map(|st| st.into());
        tracing::debug!("added file metadata to builder!");

        Ok(())
    }

    /// Grabs the format of the media file at `path`.
    #[tracing::instrument]
    async fn format(path: &Utf8Path) -> Result<(Format, Type), RavesError> {
        let path_str = path.to_string();

        tracing::debug!("grabbing format...");
        let mime = infer::get_from_path(path)
            .map_err(|_e| RavesError::MediaDoesntExist {
                path: path_str.clone(),
            })?
            .ok_or(RavesError::NoMimeData {
                path: path_str.clone(),
            })?;

        // aaaand make the format
        tracing::debug!("creating mime type for media file...");

        Ok((
            Format::new_from_mime(mime.mime_type())
                .ok_or(RavesError::FileNotSupportedMedia {
                    path: path_str.clone(),
                })
                .inspect_err(|e| tracing::error!("Failed to create MIME type! err: {e}"))?,
            mime,
        ))
    }

    /// Builds the metadata from the data gathered.
    ///
    /// This will return a None if no file metadata could be gathered.
    #[tracing::instrument(skip(self))]
    async fn build(self) -> Result<Media, RavesError> {
        let path = self.path.ok_or(RavesError::FileMissingMetadata(
            "... no path".into(),
            "no path given".into(),
        ))?;

        // if the media was previously saved in the database, we'll need to use
        // its id and 'first seen date'
        let (id, first_seen_date) = {
            let mut conn = DATABASE.acquire().await.inspect_err(|e| {
                tracing::error!("Failed to connect to database in metadata builder! err: {e}")
            })?;

            // TODO: when we start doing file hashes, we can check that too.
            // (the path is somewhat likely to change over time, but not hash!)
            let old_media_query =
                sqlx::query_as::<_, Media>(&format!("SELECT * FROM {INFO_TABLE} WHERE path = $1"))
                    .bind(&path)
                    .fetch_optional(&mut *conn)
                    .await
                    .inspect_err(|e| tracing::error!("Failed to query database! err: {e}"))?;

            if let Some(old_media) = old_media_query {
                (old_media.id, old_media.first_seen_date)
            } else {
                (Uuid::new_v4(), Utc::now())
            }
        };

        Ok(Media {
            id,

            path: path.clone(),
            filesize: self.filesize.ok_or(RavesError::FileMissingMetadata(
                path.clone(),
                "no file size given".into(),
            ))?,
            creation_date: self.creation_date,
            modification_date: self.modification_date,

            format: self.format.ok_or(RavesError::FileMissingMetadata(
                path.clone(),
                "no format given".into(),
            ))?,
            width_px: self.width_px.ok_or(RavesError::FileMissingMetadata(
                path.clone(),
                "no width (res) given".into(),
            ))?,
            height_px: self.height_px.ok_or(RavesError::FileMissingMetadata(
                path.clone(),
                "no width (res) given".into(),
            ))?,
            specific_metadata: self
                .specific_metadata
                .ok_or(RavesError::FileMissingMetadata(
                    path.clone(),
                    "no specific metadata (file kind variant)".into(),
                ))?,
            other_metadata: self.other_metadata,

            first_seen_date,

            tags: self.tags,
        })
    }
}

impl Default for MediaBuilder {
    fn default() -> Self {
        Self {
            path: None,
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
    }
}

//! # Metadata Builders
//!
//! Assists in ascertaining metadata of different kinds in one format.
//!
//! Note that this should eventually be replaced with a fleshed-out library
//! with full support for all these types!

pub mod image_crate;
pub mod kamadak;
pub mod matroska;
pub mod mp4parse;
pub mod nom;

use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::SystemTime,
};

use infer::{video::*, Type};
use tokio::io::AsyncReadExt;

use crate::{
    error::RavesError,
    models::metadata::{
        types::{Filesize, Format, MediaKind, Resolution},
        Metadata, OtherMetadataMap, SpecificMetadata,
    },
};

/// A media file's metadata. Common metadata is always present, while the `other`
/// field represents that which isn't standard in a dictionary (string, string)
/// form.
#[derive(Clone, Debug, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct MetadataBuilder {
    // # file
    pub path: Option<PathBuf>,
    pub filesize: Option<Filesize>,
    pub creation_date: Option<Option<SystemTime>>,
    pub modified_date: Option<Option<SystemTime>>,

    // # format
    pub format: Option<Format>,

    // # exif
    pub resolution: Option<Resolution>,
    pub specific: Option<SpecificMetadata>,
    pub other: Option<Option<OtherMetadataMap>>,

    // # raves-specific
    pub first_seen_date: SystemTime,
}

impl MetadataBuilder {
    /// Returns metadata found from a file.
    #[tracing::instrument]
    pub async fn apply<P>(mut self, path: P) -> Result<Metadata, RavesError>
    where
        P: AsRef<Path> + std::fmt::Debug,
    {
        let path = path.as_ref();

        // get the format
        tracing::debug!("grabbing format...");
        let (format, _inferred) = Self::format(path).await?;

        // apply format
        tracing::debug!("applying format...");
        let media_kind = format.media_kind();
        self.format = Some(format.clone());

        tracing::debug!("applying metadata...");
        match media_kind {
            MediaKind::Photo => {
                // kamadak-exif has a lot of photo formats
                let kamadak = self.apply_kamadak_exif(path, format.clone()).await;
                if kamadak.is_ok() {
                    return self.build().await;
                }

                // fallback to image crate
                tracing::warn!("couldn't get metadata from kamadak-exif. using image crate...");
                self.apply_image(path, format).await?;
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
                        path: path.display().to_string(),
                        err: e,
                    })?
                    .read_exact(&mut buf)
                    .await
                    .map_err(|e| RavesError::FileMetadataFailure {
                        path: path.display().to_string(),
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
impl MetadataBuilder {
    /// Adds typical file attributes to `self`.
    #[tracing::instrument(skip(self))]
    async fn file(&mut self, path: &Path) -> Result<(), RavesError> {
        let path_str = path.display().to_string();

        // err if the file doesn't open
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|_e| RavesError::MediaDoesntExist { path: path_str })?;
        tracing::debug!("got file metadata!");

        self.path = Some(path.to_path_buf());
        self.filesize = Some(Filesize(metadata.size()));
        self.creation_date = Some(metadata.created().ok());
        self.modified_date = Some(metadata.modified().ok());
        tracing::debug!("added file metadata to builder!");

        Ok(())
    }

    /// Grabs the format of the media file at `path`.
    #[tracing::instrument]
    async fn format(path: &Path) -> Result<(Format, Type), RavesError> {
        let path_str = path.display().to_string();

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
            Format::new_from_mime(mime.mime_type()).unwrap(),
            // Format::new_from_mime(mime.mime_type()).ok_or(RavesError::FileNotSupportedMedia {
            //     path: path_str.clone(),
            // })?,
            mime,
        ))
    }

    /// Builds the metadata from the data gathered.
    ///
    /// This will return a None if no file metadata could be gathered.
    #[tracing::instrument(skip(self))]
    async fn build(self) -> Result<Metadata, RavesError> {
        let path_str = self
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or("no path given".into());

        Ok(Metadata {
            path: self.path.ok_or(RavesError::FileMissingMetadata(
                path_str.clone(),
                "no path given".into(),
            ))?,
            filesize: self.filesize.ok_or(RavesError::FileMissingMetadata(
                path_str.clone(),
                "no file size given".into(),
            ))?,
            creation_date: self.creation_date.flatten(),
            modified_date: self.modified_date.flatten(),

            format: self.format.ok_or(RavesError::FileMissingMetadata(
                path_str.clone(),
                "no format given".into(),
            ))?,
            resolution: self.resolution.ok_or(RavesError::FileMissingMetadata(
                path_str.clone(),
                "no resolution given".into(),
            ))?,

            specific: self.specific.ok_or(RavesError::FileMissingMetadata(
                path_str.clone(),
                "no specific metadata given".into(),
            ))?,
            other: self.other.flatten(),

            // FIXME: HEYYYYY! THIS IS WRONG: MUST CHECK DATABASE!!!
            first_seen_date: self.first_seen_date,
        })
    }
}

impl Default for MetadataBuilder {
    fn default() -> Self {
        Self {
            path: None,
            resolution: None,
            filesize: None,
            format: None,
            creation_date: None,
            modified_date: None,
            first_seen_date: SystemTime::now(),
            specific: None,
            other: None,
        }
    }
}

/// Grabs the video length of a media file using FFmpeg.
pub fn get_video_len(path: &Path) -> Result<SpecificMetadata, RavesError> {
    let path_str = path.display().to_string();

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
